# Top Issues with the big-slides Code

## 1. Package Name Inconsistency ✅ COMPLETED
There's a mismatch between the name in Cargo.toml and the references in code/docs. In Cargo.toml, the package is named "big-slides" (line 2), but some old documentation referenced it as "big-presentation". This has been resolved by standardizing on "big-slides" as the package name.

## 2. Browser Detection and Error Handling
The browser rendering implementation has fragile browser detection with incomplete error handling. The code attempts to work around browser issues with retry logic (src/render.rs lines 66-98), but this approach is brittle and doesn't provide clear error messages for users about browser requirements.

## 3. Hardcoded Resource URLs
Default CSS and JS URLs are hardcoded in config.rs (lines 24-27) pointing to "https://raw.githubusercontent.com/harperreed/big/gh-pages/big.css" and related resources, which creates an external dependency that could break if these resources change or become unavailable.

## 4. PPTX Generation Uses String Concatenation for XML
The PPTX generation code (src/pptx.rs) builds XML documents through string concatenation and formatting rather than using a proper XML library. This is error-prone and makes maintenance difficult (examples in lines 73-92, 118-129, etc.).

## 5. Race Conditions in Watch Mode
The watch mode implementation lacks proper synchronization. While it attempts to prevent concurrent rebuilds with a timestamp check (watch.rs lines 419-422), this isn't sufficient to prevent race conditions when multiple file system events occur in quick succession.

## 6. Thread Safety Issues in WebSocket Implementation
The WebSocket manager (watch.rs lines 82-149) isn't fully thread-safe. It uses basic locking but doesn't handle connection cleanup and error states properly, potentially causing resource leaks or deadlocks.

## 7. Slide Counting and Navigation Reliability Issues
The slide detection and navigation logic (render.rs lines 127-215) is complex and fragile, relying on DOM manipulation through JavaScript injection. It has multiple fallback paths indicating known reliability issues, and uses hard-coded values (like defaulting to 15 slides) when detection fails.

## 8. Lack of Timeout Handling in Resource Fetching
The resource fetching code (resources.rs lines 45-80) implements retries but doesn't properly handle timeouts in all scenarios, potentially causing the application to hang when remote resources are unavailable.

## 9. Inefficient CSS/JS Processing in HTML Generation
The HTML generation (html.rs lines 22-98) embeds or links each CSS/JS file individually, potentially making multiple network requests when resource bundling would be more efficient.

## 10. Unnecessary Code Duplication
There's significant duplication between command implementations in main.rs, particularly for resource handling between generate_html (lines 192-244) and watch (lines 288-358) functions, which should be extracted into shared utility functions.
