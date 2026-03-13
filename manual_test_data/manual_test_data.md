# Manual Test Dataset Guide

This folder contains a ready-made dataset for manual testing of `fundoubler`.

Use it to validate:
- size-based matching
- hash/content matching
- name-only matching
- nested traversal
- source/search/unique logic
- regex filtering
- dry-run deletion behavior

---

## Quick Start

From project root:

```bash
cargo build --release
fundoubler manual_test_data
```

For more detail:

```bash
fundoubler manual_test_data --content -vv
```

---

## Folder Map

```text
manual_test_data/
├── by_size/
├── by_content/
├── by_name/
├── nested/
├── source_vs_search/
├── filter_candidates/
└── empty_and_small/
```

---

## 1) `by_size/` - Size vs content

Files:
- `a_100_same.txt` and `b_100_same.txt` -> same size, same content
- `c_100_diff.txt` -> same size as above, different content
- `d_200.txt` and `e_200.txt` -> same size, same content

Manual checks:

```bash
# Size only: 2 groups (100-byte trio and 200-byte pair)
fundoubler manual_test_data/by_size --size

# Default (size + xxh3): c_100_diff should be excluded from duplicate group
fundoubler manual_test_data/by_size

# Content hashing: should match true-content pairs only
fundoubler manual_test_data/by_size --content
```

Expected:
- `--size`: `a/b/c` grouped together, `d/e` grouped together
- default / `--content`: groups are `a/b` and `d/e`; `c` is unique

---

## 2) `by_content/` - Content duplicates

Files:
- `copy1.txt`, `copy2.txt`, `copy3.txt` -> identical content
- `unique.txt` -> different content

Manual checks:

```bash
fundoubler manual_test_data/by_content --md5
fundoubler manual_test_data/by_content --xxh3
fundoubler manual_test_data/by_content --content
```

Expected:
- one duplicate group with 3 files: `copy1`, `copy2`, `copy3`
- `unique.txt` not in any duplicate group

---

## 3) `by_name/` - Name-only behavior

Files:
- `sub1/readme.txt` (content one)
- `sub2/readme.txt` (content two)
- `sub1/other.txt`

Manual checks:

```bash
# Name-only: same filename in different folders should group
fundoubler manual_test_data/by_name --name

# Content-based: should not group readme files (different content)
fundoubler manual_test_data/by_name --md5
```

Expected:
- `--name`: `sub1/readme.txt` + `sub2/readme.txt` grouped
- `--md5`: no duplicates in this folder

---

## 4) `nested/` - Recursive traversal

Files:
- `level1/same.txt`
- `level1/level2/same.txt` (same content)
- `level1/solo.txt` (unique)

Manual checks:

```bash
fundoubler manual_test_data/nested --content
```

Expected:
- one group: both `same.txt` files

---

## 5) `source_vs_search/` - `--source-dir`, `--search-dir`, `--unique`

Files:
- `origin/canonical.txt`
- `downloads/copy.txt` (duplicate of canonical)
- `downloads/unique_in_downloads.txt` (unique)
- `downloads/dup_pair_a.txt`
- `other/dup_pair_b.txt` (duplicate of dup_pair_a)

Manual checks:

```bash
# Show groups touching search-dir; source files should be preferred for keep ordering
fundoubler manual_test_data/source_vs_search \
  --source-dir manual_test_data/source_vs_search/origin \
  --search-dir manual_test_data/source_vs_search/downloads \
  --content

# --unique: only groups that DO NOT contain any source-dir file
fundoubler manual_test_data/source_vs_search \
  --source-dir manual_test_data/source_vs_search/origin \
  --search-dir manual_test_data/source_vs_search/downloads \
  --unique \
  --content
```

Expected:
- without `--unique`: includes `canonical/copy` group and possibly `dup_pair_a/dup_pair_b` (if touching search rules)
- with `--unique`: excludes `canonical/copy`; keeps only groups with no file in `origin`

---

## 6) `filter_candidates/` - Regex filtering

Files:
- `image1.jpg`, `image2.jpg` (duplicates)
- `picture.png`, `picture_copy.png` (duplicates)
- `document.pdf`, `data.txt` (not duplicates in this set)

Manual checks:

```bash
# All duplicate candidates by content
fundoubler manual_test_data/filter_candidates --content

# Restrict to image formats only
fundoubler manual_test_data/filter_candidates --content --filter ".*\\.(jpg|png)$"

# Restrict to txt only
fundoubler manual_test_data/filter_candidates --content --filter ".*\\.txt$"
```

Expected:
- jpg/png filter: only jpg and png groups shown
- txt filter: no duplicates

---

## 7) `empty_and_small/` - Tiny files and min-size

Files:
- `empty.bin` (0 B)
- `one_byte.bin` (1 B)
- `tiny_a.txt` and `tiny_b.txt` (same 1-byte content)

Manual checks:

```bash
# Content mode includes tiny duplicates
fundoubler manual_test_data/empty_and_small --content

# Skip tiny files
fundoubler manual_test_data/empty_and_small --content --min-size 2
```

Expected:
- without min-size: `tiny_a` + `tiny_b` grouped
- with `--min-size 2`: no duplicates

---

## Delete / Link Dry-Run Checks

Use dry-run to validate planned actions safely:

```bash
fundoubler manual_test_data/by_content --delete --dry-run
fundoubler manual_test_data/by_content --delete --dry-run --create-symlinks
fundoubler manual_test_data/by_content --delete --dry-run --create-hardlinks
```

Expected:
- output contains `DRY RUN`
- no files are actually deleted
- for link options, output says replacement/link would be created

---

## Full-tree smoke checks

```bash
fundoubler manual_test_data
fundoubler manual_test_data --content
fundoubler manual_test_data --content --limit 2
fundoubler manual_test_data report.txt
```

Expected:
- command succeeds
- report file generated when output path is provided
- `--limit 2` shows at most 2 groups

