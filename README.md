# DuckDB Kart Extension (Rust)

A native DuckDB extension written in Rust to read geospatial datasets directly from **Kart** version-controlled repositories.

## Features

- **Pure Rust Git Engine**: Reads Table V3 datasets directly from Git trees and packfiles using `gix` (`gitoxide`) with **zero external C or dynamic library dependencies**.
- **Automatic Schema Mapping**: Inspects dataset `schema.json` and maps Kart data types (`integer`, `float`, `text`, `boolean`, `geometry`, `blob`, etc.) directly into native DuckDB types.
- **OGC Standard WKB Geometry**: Automatically parses GeoPackage binary envelopes and converts geometry fields to standard OGC Well-Known Binary (WKB) blobs, ready for use with DuckDB's `spatial` extension (`ST_GeomFromWKB(geom)`).
- **Multi-Dataset Discovery**: Inspect repositories and list available datasets, feature counts, geometry types, and coordinate reference systems (CRS).

## Installation

Download the prebuilt extension binary for your platform:

```bash
# macOS (Apple Silicon)
curl -sL "https://github.com/blacha/duckdb-kart/releases/latest/download/kart.osx_arm64.duckdb_extension" -o kart.duckdb_extension

# macOS (Intel)
curl -sL "https://github.com/blacha/duckdb-kart/releases/latest/download/kart.osx_amd64.duckdb_extension" -o kart.duckdb_extension

# Linux (x86_64)
curl -sL "https://github.com/blacha/duckdb-kart/releases/latest/download/kart.linux_amd64.duckdb_extension" -o kart.duckdb_extension
```

## Usage in DuckDB

Start DuckDB (using `-unsigned` for local extensions):

```bash
duckdb -unsigned
```

### Load the Extension

```sql
LOAD 'duckdb_kart/kart.duckdb_extension';
```

### 1. Export Dataset Directly to Parquet

Export any Kart dataset straight to Parquet in a single SQL statement:

```sql
COPY (SELECT * FROM read_kart('path/to/repo', 'dataset_name'))
TO 'dataset.parquet' (FORMAT PARQUET);
```

Or from the command line as a one-liner:

```bash
duckdb -unsigned -c "
  LOAD 'duckdb_kart/kart.duckdb_extension';
  COPY (SELECT * FROM read_kart('nz-building-outlines', 'nz_building_outlines'))
  TO 'nz_buildings.parquet' (FORMAT PARQUET);
"
```

> **Performance**: Achieves **135,000+ rows/second** sustained streaming from Git packfiles directly into compressed Parquet.

### 2. Discover Datasets in a Kart Repository

```sql
SELECT * FROM read_kart_datasets('kart-test');
```

Output:
```
┌────────────────────────────────┬──────────────┬───────────────┬──────────────┬───────────────┐
│          dataset_name          │ dataset_type │ feature_count │ geometry_crs │ geometry_type │
│            varchar             │   varchar    │     int64     │   varchar    │    varchar    │
├────────────────────────────────┼──────────────┼───────────────┼──────────────┼───────────────┤
│ nz_topo_map_sheet              │ table        │           445 │ EPSG:4167    │ POLYGON       │
│ nz_vineyard_polygons_topo_150k │ table        │          2362 │ EPSG:2193    │ MULTIPOLYGON  │
└────────────────────────────────┴──────────────┴───────────────┴──────────────┴───────────────┘
```

### 3. Query a Dataset

Pass `repo_path` and `dataset_name`:

```sql
SELECT fid, sheet_code, sheet_name, edition, octet_length(geom) as geom_wkb_bytes
FROM read_kart('kart-test', 'nz_topo_map_sheet')
WHERE sheet_name LIKE '%Auckland%' OR sheet_name LIKE '%Wellington%';
```

Output:
```
┌────────────┬────────────┬─────────────────────────────┬────────────────┐
│ sheet_code │ sheet_name │           edition           │ geom_wkb_bytes │
│  varchar   │  varchar   │           varchar           │     int64      │
├────────────┼────────────┼─────────────────────────────┼────────────────┤
│ BA32       │ Auckland   │ Edition 1.07 Published 2022 │             93 │
│ BQ31       │ Wellington │ Edition 2.04 Published 2019 │             93 │
└────────────┴────────────┴─────────────────────────────┴────────────────┘
```

Or query by joined path using `read_kart_dataset`:

```sql
SELECT count(*), min(fid), max(fid)
FROM read_kart_dataset('kart-test/nz_vineyard_polygons_topo_150k');
```
Output:
```
┌──────────────┬──────────────┬──────────────┐
│ count_star() │   min(fid)   │   max(fid)   │
│    int64     │    int64     │    int64     │
├──────────────┼──────────────┼──────────────┤
│         2362 │            1 │         2362 │
└──────────────┴──────────────┴──────────────┘
```

### 4. Spatial Queries with DuckDB Spatial Extension

When the DuckDB `spatial` extension is installed:

```sql
LOAD spatial;
SELECT fid, sheet_name, ST_AsText(ST_GeomFromWKB(geom)) as wkt
FROM read_kart('kart-test', 'nz_topo_map_sheet')
LIMIT 5;
```

---

## Building from Source

Requires Rust and Python:

```bash
cd duckdb_kart
python3 package.py
```

This compiles the release `cdylib` in Rust and appends DuckDB's 512-byte metadata footer to produce `kart.duckdb_extension`.
