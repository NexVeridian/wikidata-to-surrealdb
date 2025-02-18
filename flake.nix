{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      fenix,
      flake-utils,
      advisory-db,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p:
          (
            let
              fp = fenix.packages.${system};
              fpc = fp.complete;
            in
            (fp.combine [
              fpc.cargo
              fpc.rustc
              fpc.clippy
              fpc.rust-src
              fpc.rustc
              fpc.rustfmt
              fpc.rustc-codegen-cranelift-preview

              fp.targets.wasm32-unknown-unknown.latest.rust-std
            ])
          )
        );

        # src = craneLib.cleanCargoSource ./.;
        src =
          let
            jsonFilter = path: _type: builtins.match ".*json$" path != null;
            surqlFilter = path: _type: builtins.match ".*surql$" path != null;
            customFilter =
              path: type:
              (jsonFilter path type) || (surqlFilter path type) || (craneLib.filterCargoSources path type);
          in
          pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = customFilter;
            name = "source";
          };

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs =
            [
              # Add additional build inputs here
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              # Additional darwin specific inputs can be set here
              pkgs.libiconv
            ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        my-crate = craneLib.buildPackage (
          commonArgs
          // {
            doCheck = false;
            inherit cargoArtifacts;
          }
        );

        # Define the Docker image build
        dockerImage = pkgs.dockerTools.buildImage {
          name = "wikidata-to-surrealdb";
          tag = "latest";
          copyToRoot = [ my-crate ];
          config = {
            Cmd = [ "${my-crate}/bin/wikidata-to-surrealdb" ];
          };
        };
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit my-crate;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          my-crate-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = ''
                --all-targets -- --deny warnings -W clippy::nursery -W rust-2018-idioms \
                -A clippy::future_not_send -A clippy::option_if_let_else -A clippy::or_fun_call
              '';
            }
          );

          # my-crate-doc = craneLib.cargoDoc (commonArgs // {
          #   inherit cargoArtifacts;
          # });

          # Check formatting
          my-crate-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # # Audit dependencies
          # my-crate-audit = craneLib.cargoAudit {
          #   inherit src advisory-db;
          # };

          # # Audit licenses
          # my-crate-deny = craneLib.cargoDeny {
          #   inherit src;
          # };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `my-crate` if you do not want
          # the tests to run twice
          my-crate-nextest = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            }
          );
        };

        packages = {
          default = my-crate;
          inherit
            my-crate
            dockerImage
            ;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = my-crate;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
            # pkgs.ripgrep
          ];
        };
      }
    );
}
