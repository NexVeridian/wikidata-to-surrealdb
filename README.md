# Wikidata to SurrealDB
A tool for converting Wikidata dumps to a [SurrealDB](https://surrealdb.com/) database. Either From a bz2 or json file.

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
Copy [docker-compose.yml](./docker-compose.yml)

Create data folder next to docker-compose.yml and .env, place data inside, and set the data type in .env   
```
├── data
│   ├── Entity.json
│   ├── latest-all.json.bz2
│   └── surrealdb
├── docker-compose.yml
└── .env
```

`docker compose up --pull always -d`

## View Progress
`docker attach wikidata-to-surrealdb`

## Example .env
``` 
DB_USER=root
DB_PASSWORD=root
WIKIDATA_LANG=en
FILE_FORMAT=bz2
FILE_NAME=data/latest-all.json.bz2
# If not using docker file for Wikidata to SurrealDB, use 0.0.0.0:8000
WIKIDATA_DB_PORT=surrealdb:8000
THREADED_REQUESTS=true
# true=overwrite existing data, false=skip if already exists
OVERWRITE_DB=false
```

# [Dev Install](./CONTRIBUTING.md#dev-install)

# How to Query
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
```rust
pub enum ClaimData {
    // Entity, Property, Lexeme Tables
    Thing(Thing), 
    ClaimValueData(ClaimValueData),
}
```
#### [Docs for ClaimValueData](https://docs.rs/wikidata/0.3.1/wikidata/enum.ClaimValueData.html)

# Similar Projects
- [wd2duckdb](https://github.com/weso/wd2duckdb)
- [wd2sql](https://github.com/p-e-w/wd2sql)

# License
All code in this repository is dual-licensed under either [License-MIT](./LICENSE-MIT) or [LICENSE-APACHE](./LICENSE-Apache) at your option. This means you can select the license you prefer. [Why dual license](https://github.com/bevyengine/bevy/issues/2373).
