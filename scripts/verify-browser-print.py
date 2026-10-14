"""Opt-in PDF inspection; requires installed PyMuPDF and pypdf.

Compare actual browser print output with the UTF-8 reference emitted by the
viewport browser regression. This does not run the app or prove device behavior.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import unicodedata


def normalized(value):
    return unicodedata.normalize("NFC", " ".join(value.split()))


def verify(pdf, reference):
    import pymupdf
    from pypdf import PdfReader

    # Do not use the Windows locale: Node writes this reference as UTF-8.
    expected = json.loads(reference.read_text(encoding="utf-8", errors="strict"))
    if not expected or [row["rank"] for row in expected] != list(range(1, len(expected) + 1)):
        raise ValueError("reference must contain every rank in order")
    with pymupdf.open(pdf) as document:
        text = "\n".join(page.get_text() for page in document)
        pairs = [(int(match[1]), match[2]) for match in re.finditer(r"(?m)^(\d+) ([^\n]+)\n", text)
                 if not match[2].startswith(("skaters ", "goalies "))]
        rank_order = [rank for rank, _ in pairs] == [row["rank"] for row in expected]
        name_order = [normalized(name) for _, name in pairs] == [normalized(row["name"]) for row in expected]
        outside = [{"page": index + 1, "text": word[4]} for index, page in enumerate(document)
                   for word in page.get_text("words")
                   if word[0] < 0 or word[1] < 0 or word[2] > page.rect.width + .1 or word[3] > page.rect.height + .1]
        first_pace = expected[0]["pace"] in document[0].get_text()
        pages = len(document)
    # Independent extraction confirms accented names without terminal encoding.
    other_text = normalized("\n".join(page.extract_text() or "" for page in PdfReader(pdf).pages))
    missing = [row["name"] for row in expected if normalized(row["name"]) not in other_text]
    return {
        "verified": rank_order and name_order and first_pace and not outside and not missing,
        "pages": pages, "expected_rows": len(expected), "ranks_in_order": rank_order,
        "unicode_names_in_order": name_order, "independent_extractor_missing_names": missing,
        "first_pace_complete": first_pace, "out_of_page_text": outside,
        "reference_encoding": "utf-8", "pdf_sha256": hashlib.sha256(pdf.read_bytes()).hexdigest(),
        "reference_sha256": hashlib.sha256(reference.read_bytes()).hexdigest(),
        "scope": "Exact browser PDF text/order/bounds inspection with two parsers; not physical printer, screen-reader, phone or memory acceptance.",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pdf", type=Path)
    parser.add_argument("reference", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = verify(args.pdf, args.reference)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    # ASCII JSON is safe even when Windows redirects stdout using CP1252.
    print(json.dumps(report, ensure_ascii=True))
    raise SystemExit(0 if report["verified"] else 1)
