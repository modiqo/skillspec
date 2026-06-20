# PDF Reference

This reference contains advanced PDF examples that should be loaded only when a
task needs extraction details or library selection.

## Basic text extraction

```python
from pypdf import PdfReader
import sys

reader = PdfReader(sys.argv[1])
for page in reader.pages:
    print(page.extract_text() or "")
```

The SkillSpec preserves this as `code.extract_pdf_text`, but marks it as an
example rather than an automatic command.
