# https://mcr.microsoft.com/en-us/product/devcontainers/rust/about
FROM mcr.microsoft.com/devcontainers/rust:bookworm

RUN apt-get update && \
	apt install -y build-essential xz-utils musl-tools musl-dev gcc-multilib pkg-config libssl-dev clang mold

# RUN LAZYGIT_VERSION=$(curl -s "https://api.github.com/repos/jesseduffield/lazygit/releases/latest" | grep -Po '"tag_name": "v\K[0-9.]+') && \
# 	curl -Lo lazygit.tar.gz "https://github.com/jesseduffield/lazygit/releases/latest/download/lazygit_${LAZYGIT_VERSION}_Linux_x86_64.tar.gz" && \
# 	sudo tar xf lazygit.tar.gz -C /usr/local/bin lazygit && \ 
# 	rm -rf lazygit.tar.gz

# RUN BTOP_VERSION=$(curl -s "https://api.github.com/repos/aristocratos/btop/releases/latest" | grep -Po '"tag_name": "v\K[0-9.]+') && \ 
# 	wget "https://github.com/aristocratos/btop/releases/download/v${BTOP_VERSION}/btop-x86_64-linux-musl.tbz" && \
# 	sudo tar -xvf btop-x86_64-linux-musl.tbz && \
# 	cd btop && ./install.sh && cd .. && \
# 	rm -rf btop-x86_64-linux-musl.tbz btop

# RUN curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin
# cargo install cargo-nextest --locked
