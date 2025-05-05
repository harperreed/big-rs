# Big-Slides

A Rust-based tool for generating presentations from Markdown files. This tool provides a complete pipeline for creating PowerPoint presentations:

1. Convert Markdown to HTML with styling
2. Render HTML to slide images
3. Create PPTX presentations from the slide images

## Installation

### Prerequisites

- Rust and Cargo (https://rustup.rs/)
- A compatible browser for headless rendering (Chrome/Chromium recommended)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/big-slides.git
cd big-slides

# Build the project
cargo build --release

# The binary will be available at target/release/big
```

## Usage

Big-Slides provides three main commands:

### generate-html

Converts a Markdown file to an HTML presentation.

```bash
big generate-html -i input.md -o output.html [--css style1.css,style2.css] [--js script1.js,script2.js]
```

Options:
- `-i, --input`: Path to the markdown file
- `-o, --output`: Path to output HTML file
- `--css`: CSS files to include (local paths or URLs, comma-separated)
- `--js`: JavaScript files to include (local paths or URLs, comma-separated)
- `--mode`: Mode for CSS/JS: 'embed' to embed content or 'link' to reference (default: "embed")

### generate-slides

Renders an HTML presentation to a series of image files.

```bash
big generate-slides -i presentation.html -o slides_directory [--base-name slide] [--format png] [--width 1280] [--height 720]
```

Options:
- `-i, --input`: Path to the HTML file to render
- `-o, --output-dir`: Directory to output slide images
- `--base-name`: Base filename for slides (default: "slide")
- `--format`: Format for the slide images (default: "png")
- `--width`: Width of the slides in pixels (default: 1280)
- `--height`: Height of the slides in pixels (default: 720)

### generate-pptx

Creates a PPTX presentation from a directory of slide images.

```bash
big generate-pptx -i slides_directory -o presentation.pptx [--pattern "*.png"] [--title "My Presentation"]
```

Options:
- `-i, --input-dir`: Directory containing slide images
- `-o, --output`: Output PPTX file path
- `--pattern`: Pattern to match slide images (default: "*.png")
- `--title`: Title for the presentation (default: "Presentation")

## Full Pipeline Example

```bash
# 1. Convert markdown to HTML
big generate-html -i presentation.md -o presentation.html

# 2. Render HTML to slide images
big generate-slides -i presentation.html -o slides_dir

# 3. Create PPTX from slides
big generate-pptx -i slides_dir -o presentation.pptx
```

## Markdown Format

Big-Slides expects markdown files where slides are separated by `---`:

```markdown
# Slide 1

This is the first slide.

---

# Slide 2

This is the second slide.

---

# Slide 3

This is the third slide.
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run browser-dependent tests (ignored by default)
cargo test -- --ignored
```

## License

MIT License