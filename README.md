# Wikidata to SurrealDB
A tool for converting Wikidata dumps to a [SurrealDB](https://surrealdb.com/) database. Either From a bz2 or json file. 

The surrealdb database is ~2.6GB uncompressed or 0.5GB compressed, while the bz2 file is ~80GB, gzip file is ~130GB, and the uncompressed json file is over 1TB.

Building the database on a 7600k takes ~55 hours, using ThreadedSingle, using a cpu with more cores should be faster.

# Getting The Data
https://www.wikidata.org/wiki/Wikidata:Data_access

## From bz2 file ~80GB
### Dump: [Docs](https://www.wikidata.org/wiki/Wikidata:Database_download)
### [Download - latest-all.json.bz2](https://dumps.wikimedia.org/wikidatawiki/entities/latest-all.json.bz2)

## From json file
### Linked Data Interface: [Docs](https://www.wikidata.org/wiki/Wikidata:Data_access#Linked_Data_Interface_(URI)) 
```
https://www.wikidata.org/wiki/Special:EntityData/Q60746544.json
https://www.wikidata.org/wiki/Special:EntityData/P527.json
```

# Install
Copy [docker-compose-surrealdb.yml](./docker-compose-surrealdb.yml)

Create data folder next to docker-compose.yml and .env, place data inside, and set the data type in .env   
```
├── data
│   ├── Entity.json
│   ├── latest-all.json.bz2
│   ├── filter.surql
│   ├── surrealdb
│   └── temp
├── Makefile
├── docker-compose.yml
└── .env
```

### Then run:
`make up-surrealdb`

### Exit with:
`make down-surrealdb`

## View Progress
`make view`

## Example .env
```bash
DB_USER=root
DB_PASSWORD=root
WIKIDATA_LANG=en
WIKIDATA_FILE_FORMAT=bz2
WIKIDATA_FILE_NAME=data/latest-all.json.bz2
# If not using docker file for Wikidata to SurrealDB, use 0.0.0.0:8000
WIKIDATA_DB_PORT=surrealdb:8000
# true=overwrite existing data, false=skip if already exists
OVERWRITE_DB=false
CREATE_VERSION=Bulk
#FILTER_PATH=data/filter.surql
```

Env string CREATE_VERSION must be in the enum CREATE_VERSION
```rust
pub enum CreateVersion {
    Single,
    #[default]
    Bulk,
    /// must create a filter.surql file in the data directory
    BulkFilter,
}
```

### [filter.surql examples](./Useful%20queries.md#filter.surql-examples)

# [Dev Install](./CONTRIBUTING.md#dev-install)

# How to Query
```
namespace = wikidata
database = wikidata
```

## See [Useful queries.md](./Useful%20queries.md)

# Table Schema
## SurrealDB Thing
```rust
pub struct Thing {
    pub table: String,
    pub id: Id, // i64
}
```

## Tables: Entity, Property, Lexeme
```rust
pub struct EntityMini {
    pub id: Option<Thing>,
    pub label: String,
     // Claims Table
    pub claims: Thing,
    pub description: String,
}
```

## Table: Claims
```rust
pub struct Claim {
    pub id: Thing,
    pub value: ClaimData,
}
```

### ClaimData
#### [Docs](https://docs.rs/wikidata/0.3.1/wikidata/enum.ClaimValueData.html)
```rust
pub enum ClaimData {
    // Entity, Property, Lexeme Tables
    Thing(Thing), 
    ClaimValueData(ClaimValueData),
}
```

# Similar Projects
- [wd2duckdb](https://github.com/weso/wd2duckdb)
- [wd2sql](https://github.com/p-e-w/wd2sql)

# License
All code in this repository is dual-licensed under either [License-MIT](./LICENSE-MIT) or [LICENSE-APACHE](./LICENSE-Apache) at your option. This means you can select the license you prefer. [Why dual license](https://github.com/bevyengine/bevy/issues/2373).
