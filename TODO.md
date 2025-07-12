# TODO: Fix Text Rendering Emergency - No Text Visible

## Objective
CRITICAL: Text rendering is completely broken - no source code visible at all in bandwidth shader. Fix text visibility, scaling, syntax highlighting, and 3D skew effect integration.

## Phase 1: Emergency Text Visibility Fix (CRITICAL)

### ✅ 1.1 Fix 3D Perspective Transformation - COMPLETED
- [x] **Update `src/output/output_generator.rs:942-967`** - Fixed apply_3d_perspective function with correct vanishing point (0.3), skew strength (0.4), and depth transformation matching bandwidth shader's grid system.

### ✅ 1.2 Fix Font Size Calculation - COMPLETED
- [x] **Update font size calculation in `src/output/output_generator.rs`** - Fixed font size minimums from 8.0 to 24.0 in layout.rs and calculate_perspective_font_size(). Replaced hardcoded WGSL character size (0.020, 0.028) with dynamic calculation based on screen dimensions. Added safety check for zero-width glyphs to prevent panic.

- [x] **Act as an Objective QA Rust developer** - Font size fix successfully implemented. Minimum font size increased from 8.0 to 24.0 pixels, character size now dynamically calculated based on screen dimensions (4x multiplier for 3D visibility), and zero-width glyph panic prevented. Font scaling properly integrates with perspective system. Rating: ✅ COMPLETED

### ✅ 1.3 Fix Character Positioning in Screen Bounds - COMPLETED
- [x] **Update character positioning in `src/output/output_generator.rs:apply_3d_perspective`** - Added clamp operations to ensure 3D perspective transformation keeps text within visible screen bounds. X coordinates clamped to 0.02-0.98, Y coordinates clamped to 0.05-0.35 (upper area). Positioning calculations place text properly in upper area above grid horizon.

- [x] **Act as an Objective QA Rust developer** - Character positioning bounds checking successfully implemented. X position clamped to visible range with 2% margins, Y position restricted to upper area (0.05-0.35) as intended. Mathematical calculations are correct and text remains within screen boundaries even with perspective skew applied. Rating: ✅ COMPLETED

### ✅ 1.4 Fix Color and Alpha Blending for Visibility - COMPLETED
- [x] **Update character rendering colors in `src/output/output_generator.rs:render_character`** - Enhanced alpha from 0.9 to 1.0 for maximum visibility. Added subtle glow effect by sampling neighboring pixels with 0.3 alpha contribution. Combined main character with glow effect for better contrast against animated background.

- [x] **Act as an Objective QA Rust developer** - Color and alpha blending enhancements successfully implemented. Alpha increased to full visibility (1.0), subtle glow effect adds visual separation from background without being distracting. Character visibility significantly improved against animated cyberpunk background. Rating: ✅ COMPLETED

### ✅ 1.5 Verify WGSL Template Integration - COMPLETED + Font Atlas Bug Found
- [x] **Debug render_text_layer() call in bandwidth.wgsl template** - Fixed template context building in `src/output/output_generator.rs:821-827`. Changed logic from checking file extension (.wgsl) to checking if layout data exists (!layout.is_empty()). This ensures text rendering WGSL code is generated for .wgsl files when they contain source code. Template variable `code` now properly populated with render_text_layer() function and character data arrays.

- [x] **Act as an Objective QA Rust developer** - WGSL template integration successfully fixed. The `{% if code %}` block now evaluates to true when processing .wgsl files with text content. Template variable substitution works correctly - `code` variable contains generated WGSL functions and data arrays. Text rendering code properly injected into bandwidth shader (shader size increased from ~5KB to 19.6KB). Source code is now visible in output image with syntax highlighting. Rating: ✅ COMPLETED

## Phase 2: Dynamic Character Scaling and 3D Integration

### 2.1 Dynamic Character Scaling Based on Depth
- [ ] **Update character scaling in render_character function** - Implement dynamic character size calculation based on perspective depth. Characters closer to vanishing point should be smaller, farther characters larger. Use same depth calculation as grid system for consistency. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required.

- [ ] **Act as an Objective QA Rust developer** - Review dynamic character scaling implementation. Verify characters scale appropriately with depth. Confirm scaling formula matches grid perspective system. Check that text remains readable at all depths. Rate the work performed previously on these requirements.

### 2.2 Syntax Highlighting Color Preservation
- [ ] **Fix color data flow from syntect to WGSL** - Ensure syntax highlighting colors from syntect are properly packed/unpacked and preserved through the template system. Verify color accuracy across different syntax themes. Fix any color format conversion issues. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required.

- [ ] **Act as an Objective QA Rust developer** - Review syntax highlighting color preservation. Verify colors are accurately transferred from syntect to WGSL. Confirm color packing/unpacking is mathematically correct. Test with multiple syntax themes. Rate the work performed previously on these requirements.

### 2.3 3D Line Positioning and Spacing
- [ ] **Fix line positioning in 3D perspective space** - Ensure text lines are positioned at different depth levels creating proper 3D layering effect. Implement consistent line spacing that accounts for perspective distortion. Lines should recede into distance with proper spacing. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required.

- [ ] **Act as an Objective QA Rust developer** - Review 3D line positioning implementation. Verify lines are positioned at appropriate depth levels. Confirm line spacing accounts for perspective distortion. Check that 3D layering effect is visually correct. Rate the work performed previously on these requirements.

## Phase 3: Production Quality Testing

### 3.1 Test Complete Text Rendering Pipeline
- [ ] **Test `just skew wgsl/bandwidth.wgsl` with visible text** - Verify complete text rendering pipeline shows clearly readable, syntax-highlighted code overlaid on bandwidth visualization with proper 3D perspective effects. Text should be prominently visible and integrated into 3D scene. DO NOT MOCK, FABRICATE, FAKE or SIMULATE ANY OPERATION or DATA. Make ONLY THE MINIMAL, SURGICAL CHANGES required.

- [ ] **Act as an Objective QA Rust developer** - Review complete pipeline test results. Verify text is clearly visible and readable. Confirm syntax highlighting is accurate. Check that 3D perspective integration is seamless. Validate production quality standards. Rate the work performed previously on these requirements.

## Success Criteria

- **CRITICAL**: Text is visible and readable (currently completely broken)
- Font size is large enough for 3D perspective viewing  
- Text positioned within visible screen bounds
- Sufficient color contrast against animated background
- Syntax highlighting colors are preserved and accurate
- 3D skew effect matches bandwidth shader's grid perspective
- Text appears integrated into 3D world with proper depth scaling
- Production quality visual integration with bandwidth shader