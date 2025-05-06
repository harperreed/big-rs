# Watch Mode

The `watch` command in big-slides allows you to automatically regenerate your presentation when source files change. This is perfect for iterative development and for live presentations.

## Basic Usage

```bash
big watch -i your-markdown.md -o slides.html
```

This will:
1. Generate HTML from your markdown file
2. Watch for changes to the markdown file
3. Automatically regenerate the HTML when changes are detected

## Full Pipeline Watching

You can set up the watch command to monitor the entire pipeline:

```bash
big watch -i your-markdown.md -o slides.html --slides-dir ./slides --pptx-output presentation.pptx
```

This will:
1. Generate HTML from your markdown file
2. Generate slide images from the HTML
3. Generate a PPTX file from the slide images
4. Watch for changes to the markdown file
5. Automatically regenerate all outputs when changes are detected

## Live Preview with Web Server

Add the `--serve` flag to start a local web server:

```bash
big watch -i your-markdown.md -o slides.html --serve
```

This will:
1. Start a local web server on port 8080 (default)
2. Serve the HTML file and related resources
3. Allow you to view your slides in a browser at http://localhost:8080
4. Automatically update the slides as you make changes to the markdown

You can specify a custom port with `--port`:

```bash
big watch -i your-markdown.md -o slides.html --serve --port 3000
```

## Auto-Reload with WebSockets

For an even better development experience, enable auto-reload with the `--auto-reload` flag:

```bash
big watch -i your-markdown.md -o slides.html --serve --auto-reload
```

This will:
1. Start a local web server on port 8080 (default)
2. Start a WebSocket server on port 8081 (default is HTTP port + 1)
3. Inject a small WebSocket client script into the HTML
4. Automatically refresh the browser when changes are detected

You can specify a custom WebSocket port with `--ws-port`:

```bash
big watch -i your-markdown.md -o slides.html --serve --auto-reload --ws-port 9000
```

## Custom Styling and Interactivity

You can include custom CSS and JavaScript files:

```bash
big watch -i your-markdown.md -o slides.html --css style.css --js navigation.js --serve
```

Multiple files can be specified with commas:

```bash
big watch -i your-markdown.md -o slides.html --css base.css,theme.css,custom.css --serve
```

## Embedding vs Linking Resources

By default, CSS and JS content is embedded in the HTML. If you prefer to link to these files:

```bash
big watch -i your-markdown.md -o slides.html --css style.css --js navigation.js --mode=link --serve
```

## Remote Resources

You can use remote CSS/JS files with URLs:

```bash
big watch -i your-markdown.md -o slides.html --css https://cdn.example.com/style.css --serve
```

## Debouncing

The watch command includes debouncing to avoid excessive rebuilds. The default debounce time is 500ms. You can adjust this with the `--debounce-ms` option:

```bash
big watch -i your-markdown.md -o slides.html --debounce-ms 1000
```

## Example

Try our example presentation:

```bash
# Run the example script to see the full lifecycle
./example.sh

# Or directly use watch mode with the example presentation
./target/debug/big watch -i example-preso/slides.md -o example-preso/output/slides.html \
  --css example-preso/big-theme.css --js example-preso/big-navigation.js \
  --slides-dir example-preso/output/slides --pptx-output example-preso/output/presentation.pptx \
  --serve
```