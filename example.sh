#!/bin/bash
# Example lifecycle script for big-slides
# This script demonstrates the full pipeline from markdown to HTML to slides to PPTX

set -e  # Exit on error

# First, build the project
echo "Building big-slides..."
cargo build

# Define colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Define paths
MARKDOWN_FILE="example-preso/slides.md"
HTML_OUTPUT="example-preso/output/slides.html"
SLIDES_DIR="example-preso/output/slides"
PPTX_OUTPUT="example-preso/output/presentation.pptx"
# CSS_FILE="example-preso/big-theme.css"
# JS_FILE="example-preso/big-navigation.js"

# Create output directory
mkdir -p "example-preso/output/slides"

echo -e "${BLUE}=== Big-Slides Example Lifecycle ===${NC}\n"

# Step 1: Generate HTML from Markdown
echo -e "${YELLOW}Step 1: Generating HTML from Markdown${NC}"
echo -e "Command: ./target/debug/big generate-html -i ${MARKDOWN_FILE} -o ${HTML_OUTPUT}"
./target/debug/big generate-html -i ${MARKDOWN_FILE} -o ${HTML_OUTPUT}
echo -e "${GREEN}✓ HTML generated successfully with custom CSS and JS${NC}\n"

# Step 2: Generate slides (images) from HTML
echo -e "${YELLOW}Step 2: Generating slides from HTML${NC}"
echo -e "Command: ./target/debug/big generate-slides -i ${HTML_OUTPUT} -o ${SLIDES_DIR}"
./target/debug/big generate-slides -i ${HTML_OUTPUT} -o ${SLIDES_DIR}
echo -e "${GREEN}✓ Slides generated successfully${NC}\n"

# Step 3: Generate PPTX from slides
echo -e "${YELLOW}Step 3: Generating PPTX from slides${NC}"
echo -e "Command: ./target/debug/big generate-pptx -i ${SLIDES_DIR} -o ${PPTX_OUTPUT}"
./target/debug/big generate-pptx -i ${SLIDES_DIR} -o ${PPTX_OUTPUT}
echo -e "${GREEN}✓ PPTX generated successfully${NC}\n"

# Note: For detailed watch mode instructions, see WATCH_MODE.md
echo -e "${BLUE}Note: See WATCH_MODE.md for watch mode instructions${NC}\n"

# Step 5: Adding style notes
echo -e "${YELLOW}Step 5: Customizing your presentation${NC}"
echo -e "${BLUE}Custom CSS and JS:${NC}"
echo -e "  ${GREEN}--css ${CSS_FILE}${NC} - Apply custom styling"
echo -e "  ${GREEN}--js ${JS_FILE}${NC} - Add navigation and interactivity"
echo -e ""
echo -e "${BLUE}Multiple files:${NC}"
echo -e "  You can specify multiple CSS or JS files by separating them with commas:"
echo -e "  ${GREEN}--css file1.css,file2.css,file3.css${NC}"
echo -e "  ${GREEN}--js file1.js,file2.js,file3.js${NC}"
echo -e ""
echo -e "${BLUE}Embedding vs Linking:${NC}"
echo -e "  By default, CSS and JS content is embedded in the HTML. Use --mode=link to reference files instead:"
echo -e "  ${GREEN}--mode=link${NC}"
echo -e ""
echo -e "${BLUE}Remote resources:${NC}"
echo -e "  You can use remote CSS/JS files with URLs:"
echo -e "  ${GREEN}--css https://cdn.example.com/style.css${NC}\n"

echo -e "${GREEN}All steps completed successfully!${NC}"
echo -e "${BLUE}Generated files:${NC}"
echo -e "  HTML: ${HTML_OUTPUT}"
echo -e "  Slides: ${SLIDES_DIR}"
echo -e "  PPTX: ${PPTX_OUTPUT}"
