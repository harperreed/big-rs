# Big-Slides

A Rust-based tool for generating professional PowerPoint presentations from Markdown files. This tool provides a complete pipeline for creating presentations:

1. **Convert Markdown to HTML** with custom styling and embedded scripts
2. **Render HTML to slide images** using headless browser automation
3. **Create PowerPoint (PPTX)** presentations from the slide images

## Features

- **Simple Markdown Syntax**: Write presentations in easy-to-use Markdown
- **Custom Styling**: Include CSS to control the look and feel of your slides
- **JavaScript Support**: Add interactivity with custom JS
- **Efficient Pipeline**: Generate presentations in a three-step process
- **Local and Remote Resources**: Include local files or remote URLs for CSS/JS
- **Customizable Output**: Control image dimensions, format, and naming
- **Complete PPTX Generation**: Creates fully-functional PowerPoint files

## Installation

### Prerequisites

- Rust and Cargo (https://rustup.rs/)
- Chrome/Chromium for headless rendering
  - On macOS: `brew install --cask google-chrome`
  - On Ubuntu/Debian: `sudo apt install chromium-browser`
  - On Windows: Download from [https://www.google.com/chrome/](https://www.google.com/chrome/)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/big-slides.git
cd big-slides

# Build the project
cargo build --release

# The binary will be available at target/release/big
```

### Verifying Installation

```bash
# Display help
./target/release/big --help

# Check version
./target/release/big --version
```

## Usage

Big-Slides provides three main commands that can be used separately or in sequence:

### 1. Generate HTML from Markdown

```bash
big generate-html -i input.md -o output.html [--css style1.css,style2.css] [--js script1.js,script2.js]
```

#### Options

- `-i, --input`: Path to the markdown file
- `-o, --output`: Path to output HTML file
- `--css`: CSS files to include (local paths or URLs, comma-separated)
- `--js`: JavaScript files to include (local paths or URLs, comma-separated)
- `--mode`: Mode for CSS/JS: 'embed' to embed content or 'link' to reference (default: "embed")

#### Example

```bash
# Basic usage
big generate-html -i presentation.md -o presentation.html

# With custom styling
big generate-html -i presentation.md -o presentation.html --css style.css --js navigation.js
```

### 2. Generate Slides from HTML

```bash
big generate-slides -i presentation.html -o slides_directory [--base-name slide] [--format png] [--width 1920] [--height 1080]
```

#### Options

- `-i, --input`: Path to the HTML file to render
- `-o, --output-dir`: Directory to output slide images
- `--base-name`: Base filename for slides (default: "slide")
- `--format`: Format for the slide images (default: "png")
- `--width`: Width of the slides in pixels (default: 1920)
- `--height`: Height of the slides in pixels (default: 1080)

#### Example

```bash
# Basic usage
big generate-slides -i presentation.html -o ./slides

# With custom dimensions and format
big generate-slides -i presentation.html -o ./slides --width 1920 --height 1080 --format png
```

### 3. Generate PPTX from Slides

```bash
big generate-pptx -i slides_directory -o presentation.pptx [--pattern "*.png"] [--title "My Presentation"]
```

#### Options

- `-i, --input-dir`: Directory containing slide images
- `-o, --output`: Output PPTX file path
- `--pattern`: Pattern to match slide images (default: "*.png")
- `--title`: Title for the presentation (default: "Presentation")

#### Example

```bash
# Basic usage
big generate-pptx -i ./slides -o presentation.pptx

# With custom title and pattern
big generate-pptx -i ./slides -o presentation.pptx --title "Quarterly Report" --pattern "slide_*.png"
```

## Full Pipeline Example

The true power of big-slides comes from running the complete pipeline:

```bash
# Create a working directory
mkdir my_presentation && cd my_presentation

# Write your presentation in markdown
cat > slides.md << EOF
# My Presentation

## Created with big-slides

---

# Key Point 1

* Easy to write
* Version controllable
* Looks great

---

# Code Example

\`\`\`rust
fn main() {
    println!("Hello, presentations!");
}
\`\`\`

---

# Thank You!

Questions?
EOF

# Create a custom style
cat > style.css << EOF
body {
  font-family: 'Arial', sans-serif;
  color: #333;
}
h1 {
  color: #0066cc;
}
.slides > div {
  background: linear-gradient(to bottom, #ffffff, #f0f0f0);
  padding: 40px;
  border-radius: 5px;
}
EOF

# 1. Convert markdown to HTML with your style
big generate-html -i slides.md -o slides.html --css style.css

# 2. Render HTML to slide images
big generate-slides -i slides.html -o ./slide_images --width 1920 --height 1080

# 3. Create PPTX from slides
big generate-pptx -i ./slide_images -o presentation.pptx --title "My Big Presentation"

# Open the presentation
open presentation.pptx  # On macOS
# Or: xdg-open presentation.pptx  # On Linux
# Or: start presentation.pptx  # On Windows
```

## Markdown Format

Big-Slides expects markdown files where slides are separated by `---` (horizontal rule):

```markdown
# Title Slide

Presentation subtitle

---

# Slide 1: Introduction

* Bullet point 1
* Bullet point 2
* Bullet point 3

---

# Slide 2: Code Example

```rust
fn main() {
    println!("Hello, world!");
}
```

---

# Slide 3: Tables

| Column 1 | Column 2 | Column 3 |
|----------|----------|----------|
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |
```

## Customizing Slides

### CSS Styling

You can create a custom CSS file to control the appearance of your slides:

```css
/* Basic styling */
body {
  font-family: 'Helvetica Neue', Arial, sans-serif;
  color: #333;
  background-color: #fff;
}

/* Title styling */
h1 {
  color: #0066cc;
  border-bottom: 2px solid #eee;
  padding-bottom: 10px;
}

/* Slide container styling */
.slides > div {
  box-shadow: 0 5px 15px rgba(0,0,0,0.1);
  margin-bottom: 20px;
  padding: 40px;
  border-radius: 5px;
}

/* Code blocks */
pre code {
  background-color: #f5f5f5;
  display: block;
  padding: 15px;
  border-radius: 5px;
  overflow-x: auto;
}

/* Tables */
table {
  border-collapse: collapse;
  width: 100%;
}

th, td {
  border: 1px solid #ddd;
  padding: 8px;
}

th {
  background-color: #f2f2f2;
}
```

### JavaScript for Navigation

You can add JavaScript to control slide navigation (especially useful if you want to preview your slides in a browser):

```javascript
document.addEventListener('DOMContentLoaded', function() {
  const slides = document.querySelectorAll('.slides > div');
  let currentSlide = 0;
  
  // Hide all slides except the first one
  for (let i = 1; i < slides.length; i++) {
    slides[i].style.display = 'none';
  }
  
  // Add keyboard navigation
  document.addEventListener('keydown', function(e) {
    if (e.key === 'ArrowRight' && currentSlide < slides.length - 1) {
      slides[currentSlide].style.display = 'none';
      currentSlide++;
      slides[currentSlide].style.display = 'block';
    } else if (e.key === 'ArrowLeft' && currentSlide > 0) {
      slides[currentSlide].style.display = 'none';
      currentSlide--;
      slides[currentSlide].style.display = 'block';
    }
  });
});
```

## Development

### Project Structure

```
big/
├── src/
│   ├── main.rs    # CLI interface and command handling
│   ├── lib.rs     # Core functionality implementation
│   └── tests.rs   # Unit tests
└── tests/
    ├── integration_test.rs       # Basic integration tests
    ├── generate_html_test.rs     # HTML generation tests
    ├── generate_slides_test.rs   # Slide generation tests
    ├── generate_pptx_test.rs     # PPTX generation tests
    └── end_to_end_test.rs        # Full pipeline tests
```

### Running Tests

```bash
# Run all non-browser tests
cargo test

# Run specific test
cargo test test_generate_html_basic

# Run browser-dependent tests (ignored by default)
cargo test -- --ignored

# Run all tests with full output
cargo test -- --nocapture
```

### Environment Variables

- `RUST_LOG`: Controls logging level (e.g., `RUST_LOG=info`)
- `BROWSER_PATH`: Specify custom browser path for testing

## Troubleshooting

### Common Issues

1. **Chrome/Chromium not found**:
   - Ensure Chrome/Chromium is installed and accessible in your PATH
   - Consider specifying browser path via environment variable

2. **Permission Issues**:
   - Make sure you have write permissions for the output directories

3. **Slide Navigation Problems**:
   - Check that your JavaScript correctly handles slide transitions
   - Ensure your HTML structure matches the expected `.slides > div` pattern

## License

MIT License

---

## Contributing

Contributions are welcome! Feel free to submit issues or pull requests.