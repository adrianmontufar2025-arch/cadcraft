//! A table made in AutoCAD keeps its formulas. AutoCAD saves a formula cell as a field: the cell
//! points (group 344) at a FIELD container (`_text`, `%<\_FldIdx 0>%`) whose child FIELD is an
//! `AcExpr` (`\AcExpr (A1+B1) \f "%lu2%pr2"`), and the value it showed is the container's 301.
//! It reads as the formula `=A1+B1` with that value. Synthetic file, written from the DXF Reference.

use cadcraft_doc::EntityKind;

fn field(handle: &str, owner: &str, evaluator: &str, code: &str, children: &[&str], value: &str) -> String {
    let mut s = format!("0\nFIELD\n5\n{handle}\n330\n{owner}\n100\nAcDbField\n1\n{evaluator}\n2\n{code}\n90\n{}\n", children.len());
    for c in children {
        s.push_str(&format!("360\n{c}\n"));
    }
    s.push_str(&format!("97\n0\n91\n63\n92\n0\n94\n2\n95\n2\n96\n0\n93\n0\n7\nACFD_FIELD_VALUE\n301\n{value}\n98\n{}\n", value.len()));
    s
}

fn dxf() -> String {
    let cell = |inner: &str| format!("171\n1\n172\n0\n173\n0\n174\n0\n175\n1\n176\n1\n91\n0\n178\n0\n145\n0\n{inner}");
    let mut s = String::from(
        "0\nSECTION\n2\nENTITIES\n0\nACAD_TABLE\n5\n100\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockReference\n2\n*T1\n10\n0\n20\n0\n30\n0\n100\nAcDbTable\n91\n2\n92\n3\n141\n2\n141\n2\n142\n10\n142\n10\n142\n10\n",
    );
    for inner in ["1\n2\n", "1\n3\n", "344\n200\n", "1\n5\n", "1\n7\n", "344\n210\n"] {
        s.push_str(&cell(inner));
    }
    s.push_str("0\nENDSEC\n0\nSECTION\n2\nOBJECTS\n");
    s.push_str(&field("200", "100", "_text", "%<\\_FldIdx 0>%", &["201"], "5.00"));
    s.push_str(&field("201", "200", "AcExpr", "\\AcExpr (A1+B1) \\f \"%lu2%pr2\"", &[], "5.00"));
    s.push_str(&field("210", "100", "_text", "%<\\_FldIdx 0>%", &["211"], "17"));
    s.push_str(&field("211", "210", "AcExpr", "\\AcExpr (Sum(A1:B2))", &[], "17"));
    s.push_str("0\nENDSEC\n0\nEOF\n");
    s
}

#[test]
fn an_autocad_cell_formula_reads_as_a_formula_with_the_value_it_showed() {
    let d = cadcraft_io::read(dxf().as_bytes(), "cuadro.dxf").unwrap();
    let t = d.model.iter().find_map(|e| if let EntityKind::Table(t) = &e.kind { Some(t.clone()) } else { None }).unwrap();
    assert_eq!((t.cells[0][0].text.as_str(), t.cells[1][1].text.as_str()), ("2", "7"));
    assert_eq!(t.cells[0][2].formula.as_deref(), Some("=A1+B1"));
    assert_eq!(t.cells[0][2].text, "5.00", "the value AutoCAD showed");
    assert_eq!(t.cells[1][2].formula.as_deref(), Some("=Sum(A1:B2)"));
    assert_eq!(t.cells[1][2].text, "17");
    assert!(t.cells[0][0].formula.is_none());
}
