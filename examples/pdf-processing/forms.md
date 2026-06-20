# PDF Forms

Form work is an ordered procedure. Do not fill blindly.

## Detect fillable fields

```python
from pypdf import PdfReader
import json
import sys

reader = PdfReader(sys.argv[1])
fields = reader.get_fields() or {}
print(json.dumps({
    "fillable": bool(fields),
    "field_count": len(fields),
    "fields": sorted(fields.keys()),
}, indent=2))
```

If the report says the PDF is fillable, use the existing field path.
Otherwise, fall back to reviewed overlay guidance.

## Fill existing fields

```python
from pypdf import PdfReader, PdfWriter
import json
import sys

input_pdf, values_json, output_pdf = sys.argv[1:4]
reader = PdfReader(input_pdf)
writer = PdfWriter()
writer.append(reader)
with open(values_json, "r", encoding="utf-8") as handle:
    values = json.load(handle)
for page in writer.pages:
    writer.update_page_form_field_values(page, values)
with open(output_pdf, "wb") as handle:
    writer.write(handle)
```

Validate the generated PDF before reporting completion.
