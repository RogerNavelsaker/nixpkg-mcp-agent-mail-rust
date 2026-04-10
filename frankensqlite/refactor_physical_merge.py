import re

with open('crates/fsqlite-mvcc/src/physical_merge.rs', 'r') as f:
    content = f.read()

# 1. ParsedCell
content = re.sub(r'pub struct ParsedCell \{', "pub struct ParsedCell<'a> {", content)
content = re.sub(r'pub cell_bytes: Vec<u8>,', "pub cell_bytes: &'a [u8],", content)

# 2. ParsedPage
content = re.sub(r'pub struct ParsedPage \{', "pub struct ParsedPage<'a> {", content)
content = re.sub(r'pub cells: Vec<ParsedCell>,', "pub cells: Vec<ParsedCell<'a>>,", content)

# 3. CellExtract
content = re.sub(r'type CellExtract = \(Vec<u8>, Option<i64>, \[u8; 16\]\);', "type CellExtract<'a> = (&'a [u8], Option<i64>, [u8; 16]);", content)

# 4. parse_btree_page
content = re.sub(r'pub fn parse_btree_page\(\n    page: &\[u8\],\n    page_size: PageSize,\n    reserved_per_page: u8,\n    is_page1: bool,\n    btree_ref: BtreeRef,\n\) -> Result<ParsedPage, MergeError> \{', "pub fn parse_btree_page<'a>(\n    page: &'a [u8],\n    page_size: PageSize,\n    reserved_per_page: u8,\n    is_page1: bool,\n    btree_ref: BtreeRef,\n) -> Result<ParsedPage<'a>, MergeError> {", content)

# 5. extract_cell_with_digest
content = re.sub(r'fn extract_cell_with_digest\(\n    page: &\[u8\],\n    cell_offset: usize,\n    usable: usize,\n    page_type: BTreePageType,\n    btree_ref: BtreeRef,\n\) -> Result<CellExtract, MergeError> \{', "fn extract_cell_with_digest<'a>(\n    page: &'a [u8],\n    cell_offset: usize,\n    usable: usize,\n    page_type: BTreePageType,\n    btree_ref: BtreeRef,\n) -> Result<CellExtract<'a>, MergeError> {", content)

# 6. parse_leaf_table_cell
content = re.sub(r'fn parse_leaf_table_cell\(\n    data: &\[u8\],\n    btree_ref: BtreeRef,\n    usable: u32,\n\) -> Result<CellExtract, MergeError> \{', "fn parse_leaf_table_cell<'a>(\n    data: &'a [u8],\n    btree_ref: BtreeRef,\n    usable: u32,\n) -> Result<CellExtract<'a>, MergeError> {", content)
content = re.sub(r'let cell_bytes = data\[\.\.cell_end\]\.to_vec\(\);', "let cell_bytes = &data[..cell_end];", content)

# 7. parse_leaf_index_cell
content = re.sub(r'fn parse_leaf_index_cell\(\n    data: &\[u8\],\n    btree_ref: BtreeRef,\n    usable: u32,\n\) -> Result<CellExtract, MergeError> \{', "fn parse_leaf_index_cell<'a>(\n    data: &'a [u8],\n    btree_ref: BtreeRef,\n    usable: u32,\n) -> Result<CellExtract<'a>, MergeError> {", content)

# 8. parse_interior_table_cell
content = re.sub(r'fn parse_interior_table_cell\(data: &\[u8\], btree_ref: BtreeRef\) -> Result<CellExtract, MergeError> \{', "fn parse_interior_table_cell<'a>(data: &'a [u8], btree_ref: BtreeRef) -> Result<CellExtract<'a>, MergeError> {", content)

# 9. parse_interior_index_cell
content = re.sub(r'fn parse_interior_index_cell\(\n    data: &\[u8\],\n    btree_ref: BtreeRef,\n    usable: u32,\n\) -> Result<CellExtract, MergeError> \{', "fn parse_interior_index_cell<'a>(\n    data: &'a [u8],\n    btree_ref: BtreeRef,\n    usable: u32,\n) -> Result<CellExtract<'a>, MergeError> {", content)

# 10. diff_parsed_pages
content = re.sub(r'pub fn diff_parsed_pages\(\n    base: &ParsedPage,\n    modified: &ParsedPage,\n\) -> Result<StructuredPagePatch, MergeError> \{', "pub fn diff_parsed_pages(\n    base: &ParsedPage<'_>,\n    modified: &ParsedPage<'_>,\n) -> Result<StructuredPagePatch, MergeError> {", content)
content = re.sub(r'HashMap<\[u8; 16\], &ParsedCell>', "HashMap<[u8; 16], &ParsedCell<'_>>", content)
content = re.sub(r'CellOpKind::Insert \{ cell_bytes: c\.cell_bytes\.clone\(\) \}', "CellOpKind::Insert { cell_bytes: c.cell_bytes.to_vec() }", content)
content = re.sub(r'CellOpKind::Replace \{ new_cell_bytes: c\.cell_bytes\.clone\(\) \}', "CellOpKind::Replace { new_cell_bytes: c.cell_bytes.to_vec() }", content)

# 11. apply_patch
content = re.sub(r'pub fn apply_patch\(\n    base: &ParsedPage,\n    patch: &StructuredPagePatch,\n\) -> Result<Vec<ParsedCell>, MergeError> \{', "pub fn apply_patch<'a>(\n    base: &ParsedPage<'a>,\n    patch: &'a StructuredPagePatch,\n) -> Result<Vec<ParsedCell<'a>>, MergeError> {", content)
content = re.sub(r'BTreeMap<\[u8; 16\], ParsedCell>', "BTreeMap<[u8; 16], ParsedCell<'a>>", content)
content = re.sub(r'Vec<ParsedCell>', "Vec<ParsedCell<'a>>", content)
content = re.sub(r'cell_bytes: cell_bytes\.clone\(\)', "cell_bytes: cell_bytes", content)
content = re.sub(r'cell\.cell_bytes = new_cell_bytes\.clone\(\);', "cell.cell_bytes = new_cell_bytes;", content)

# 12. repack_btree_page
content = re.sub(r'pub fn repack_btree_page\(\n    cells: &\[ParsedCell\],', "pub fn repack_btree_page(\n    cells: &[ParsedCell<'_>],", content)

with open('crates/fsqlite-mvcc/src/physical_merge.rs', 'w') as f:
    f.write(content)

