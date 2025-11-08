# Manual Testing Checklist

This document provides a comprehensive manual testing checklist for the AutoQAC Rust/Slint implementation. Use this checklist to verify functionality before releases and after significant changes.

## Test Environment Setup

Before beginning testing, ensure you have:

- [ ] Windows 10/11 system (or appropriate OS)
- [ ] xEdit executable (FO4Edit, SSEEdit, etc.)
- [ ] Bethesda game installation with plugins
- [ ] Load order file (plugins.txt or loadorder.txt)
- [ ] Optional: Mod Organizer 2 installation
- [ ] Test plugins with known ITMs/UDRs (or create test cases)

## 1. Initial Configuration Testing

### 1.1 First Launch
- [ ] Application launches successfully
- [ ] Window displays properly with Fluent Design theme
- [ ] All UI elements visible and properly laid out
- [ ] No console errors or warnings
- [ ] Configuration indicators show "Not Configured" state

### 1.2 Load Order Configuration
- [ ] Browse button opens file picker
- [ ] Can select valid plugins.txt or loadorder.txt
- [ ] Path displays in text field after selection
- [ ] Configuration indicator updates to "Configured" (checkmark)
- [ ] Invalid file path shows error feedback
- [ ] Empty/non-existent file handled gracefully
- [ ] Game type auto-detected from load order file
- [ ] Detected game type displayed correctly

### 1.3 xEdit Executable Configuration
- [ ] Browse button opens file picker filtered to .exe files
- [ ] Can select game-specific xEdit (FO4Edit, SSEEdit, etc.)
- [ ] Can select universal xEdit (xEdit.exe)
- [ ] Path displays correctly
- [ ] Configuration indicator updates to "Configured"
- [ ] Game type auto-detected from executable name
- [ ] For universal xEdit, game type from load order takes precedence
- [ ] Invalid executable path shows error feedback
- [ ] Non-executable files rejected

### 1.4 MO2 Configuration (Optional)
- [ ] MO2 browse button opens file picker
- [ ] Can select ModOrganizer.exe
- [ ] Path displays correctly
- [ ] Configuration indicator updates when set
- [ ] Empty MO2 path allowed (optional feature)
- [ ] Invalid MO2 path shows error feedback
- [ ] MO2 indicator shows "Optional" state when not configured

### 1.5 Full Configuration Check
- [ ] All three paths configured → "Ready to Clean" button enabled
- [ ] Missing any required path → button remains disabled
- [ ] Status message updates based on configuration state
- [ ] Refresh button updates configuration state
- [ ] Configuration persists across application restarts

## 2. Plugin Selection Testing

### 2.1 Load Order Display
- [ ] Plugin list populates from load order file
- [ ] Master files (.esm) displayed
- [ ] Plugin files (.esp) displayed
- [ ] Light plugins (.esl) displayed if applicable
- [ ] Plugins displayed in correct load order
- [ ] Plugin count displayed correctly
- [ ] Empty load order handled gracefully

### 2.2 Plugin Filtering
- [ ] Can select individual plugins
- [ ] Can select multiple plugins
- [ ] Can select all plugins
- [ ] Can deselect all plugins
- [ ] Search/filter functionality works (if implemented)
- [ ] Selection count updates correctly

### 2.3 Skip List Integration
- [ ] Base game files automatically identified
- [ ] Skip list plugins show visual indicator
- [ ] Can view which plugins will be skipped
- [ ] Skip list matches game type
- [ ] Custom ignore list integrated (PACT Ignore.yaml)
- [ ] Can override skip list (if feature implemented)

## 3. Cleaning Workflow Testing

### 3.1 Basic Cleaning
- [ ] Select single plugin to clean
- [ ] Click "Start Cleaning" button
- [ ] Progress dialog appears
- [ ] Plugin name displayed in progress
- [ ] xEdit process launches successfully
- [ ] Progress updates during cleaning
- [ ] xEdit closes automatically after cleaning
- [ ] Results displayed after completion
- [ ] Statistics populated correctly (ITMs, UDRs, Navmeshes)

### 3.2 Batch Cleaning
- [ ] Select multiple plugins (5-10 plugins)
- [ ] Start cleaning
- [ ] Plugins process sequentially
- [ ] Progress bar updates for each plugin
- [ ] Current plugin name displayed
- [ ] "X of Y" plugin counter updates
- [ ] Can see which plugins completed successfully
- [ ] Can see which plugins failed
- [ ] Can see which plugins were skipped
- [ ] Total statistics aggregate correctly

### 3.3 Partial Forms Mode
- [ ] Enable "Partial Forms" checkbox
- [ ] Warning dialog appears
- [ ] Warning explains risks clearly
- [ ] Can accept warning and continue
- [ ] Can cancel and disable partial forms
- [ ] xEdit command includes partial forms flags
- [ ] Partial forms count displayed in results

### 3.4 MO2 Integration
- [ ] With MO2 configured, cleaning starts correctly
- [ ] xEdit launches through MO2
- [ ] MO2 "run" command used in subprocess
- [ ] VFS-managed plugins handled correctly
- [ ] Results match non-MO2 cleaning

### 3.5 Timeout Handling
- [ ] Plugin exceeds timeout (configure short timeout for testing)
- [ ] Timeout error displayed
- [ ] Process terminated gracefully
- [ ] Can continue with remaining plugins
- [ ] Timeout value configurable (default 300s)

### 3.6 Cancel/Stop Operation
- [ ] Click "Stop" button during cleaning
- [ ] Current xEdit process terminates
- [ ] Remaining plugins not processed
- [ ] Partial results displayed
- [ ] Can start new cleaning session after stop

## 4. Results Display Testing

### 4.1 Individual Plugin Results
- [ ] Plugin name displayed
- [ ] Status badge shows correct state (Cleaned/Failed/Skipped)
- [ ] Status badge color-coded correctly:
  - [ ] Green for Cleaned
  - [ ] Red for Failed
  - [ ] Gray for Skipped
- [ ] ITM count displayed
- [ ] UDR count displayed
- [ ] Deleted Navmesh count displayed
- [ ] Partial forms count displayed (if enabled)
- [ ] Message text descriptive

### 4.2 Summary Statistics
- [ ] Total plugins processed count correct
- [ ] Total ITMs removed aggregated correctly
- [ ] Total UDRs undeleted aggregated correctly
- [ ] Total navmeshes deleted aggregated correctly
- [ ] Total partial forms aggregated correctly (if enabled)
- [ ] Total records processed calculated correctly
- [ ] Statistics card visible and readable
- [ ] Color-coded badge indicators work

### 4.3 Progress Updates
- [ ] Real-time progress updates during cleaning
- [ ] Progress bar percentage accurate
- [ ] Plugin counter updates correctly (X/Y)
- [ ] Current plugin name displayed
- [ ] Current operation status shown
- [ ] Time elapsed displayed (if implemented)
- [ ] Estimated time remaining (if implemented)

## 5. Error Handling Testing

### 5.1 Configuration Errors
- [ ] Missing load order file detected
- [ ] Missing xEdit executable detected
- [ ] Missing MO2 executable detected (if configured)
- [ ] Invalid file paths rejected
- [ ] Non-executable files rejected
- [ ] Error messages clear and helpful
- [ ] Can recover from error state

### 5.2 Runtime Errors
- [ ] xEdit fails to launch → error displayed
- [ ] xEdit crashes mid-cleaning → error handled
- [ ] Load order file deleted during operation → error handled
- [ ] Disk space issues → error displayed
- [ ] Permissions issues → error displayed
- [ ] Plugin file locked → error handled
- [ ] Error dialog shows descriptive messages
- [ ] Can continue cleaning remaining plugins after error

### 5.3 Parse Errors
- [ ] Malformed xEdit output handled gracefully
- [ ] Missing log files handled
- [ ] Corrupted log files handled
- [ ] Unknown game type handled
- [ ] Invalid plugin data handled

### 5.4 Network/File System Errors
- [ ] Network drive disconnects → error handled
- [ ] External drive removed → error handled
- [ ] Antivirus blocks xEdit → error displayed
- [ ] File permission denied → error displayed

## 6. Edge Cases Testing

### 6.1 Plugin Edge Cases
- [ ] Plugin with no ITMs/UDRs (clean plugin)
- [ ] Plugin with very high ITM count (1000+)
- [ ] Plugin with special characters in name
- [ ] Plugin with spaces in name
- [ ] Plugin with unicode characters in name
- [ ] Master file cleaning
- [ ] Light plugin cleaning (.esl)
- [ ] Very large plugin file (>100MB)
- [ ] Plugin in deeply nested folder path

### 6.2 Load Order Edge Cases
- [ ] Empty load order file
- [ ] Load order with comments
- [ ] Load order with disabled plugins
- [ ] Load order with missing plugins
- [ ] Load order with duplicate entries
- [ ] Very large load order (500+ plugins)
- [ ] Load order with only master files
- [ ] Load order with prefix characters (+, -, *)

### 6.3 Path Edge Cases
- [ ] Path with spaces
- [ ] Path with special characters
- [ ] Very long path (>260 characters - Windows limit)
- [ ] UNC network path (\\\\server\\share)
- [ ] Path with unicode characters
- [ ] Relative paths vs absolute paths
- [ ] Paths with forward slashes vs backslashes

### 6.4 Configuration Edge Cases
- [ ] All games' xEdit variants tested (FO3, FNV, FO4, SSE)
- [ ] VR variants tested (FO4VR, SkyrimVR)
- [ ] TTW (Tale of Two Wastelands) variant tested
- [ ] Universal xEdit with all game types
- [ ] Switch between game types
- [ ] Multiple xEdit versions (32-bit, 64-bit)

## 7. UI/UX Testing

### 7.1 Window Management
- [ ] Window can be resized
- [ ] Window minimum size enforced
- [ ] Window can be maximized
- [ ] Window can be minimized
- [ ] Window can be closed
- [ ] Close confirmation appears if cleaning in progress
- [ ] Window position persists across restarts (if implemented)
- [ ] Window size persists across restarts (if implemented)

### 7.2 Keyboard Navigation
- [ ] Tab navigation works through all controls
- [ ] Enter key activates default button
- [ ] Escape key closes dialogs
- [ ] Keyboard shortcuts work (if implemented)
- [ ] Focus indicators visible
- [ ] Accessible navigation for screen readers (if implemented)

### 7.3 Visual Design
- [ ] Fluent Design theme applied correctly
- [ ] Colors consistent with design system
- [ ] Proper contrast for readability
- [ ] Icons display correctly
- [ ] Status badges readable
- [ ] Progress indicators smooth
- [ ] No visual glitches or artifacts
- [ ] UI scales properly on high-DPI displays

### 7.4 Responsiveness
- [ ] UI updates during long operations
- [ ] No freezing or hanging
- [ ] Progress updates smooth
- [ ] Can interact with UI during background operations
- [ ] Cancel/Stop button responsive

## 8. Configuration Persistence Testing

### 8.1 YAML Configuration
- [ ] User config saves to "AutoQAC Data/AutoQAC Config.yaml"
- [ ] Main config loads from "AutoQAC Data/AutoQAC Main.yaml"
- [ ] Ignore config loads from "PACT Ignore.yaml"
- [ ] Configuration directory created if missing
- [ ] YAML files formatted correctly
- [ ] Invalid YAML handled gracefully
- [ ] Can edit YAML files manually
- [ ] Manual edits reflected on app restart

### 8.2 Legacy Migration
- [ ] Legacy "PACT Settings.yaml" detected
- [ ] Legacy settings migrated correctly
- [ ] All legacy fields mapped to new format
- [ ] Legacy file preserved (not deleted)
- [ ] Migration happens automatically on first run

### 8.3 Settings Persistence
- [ ] Load order path persists
- [ ] xEdit path persists
- [ ] MO2 path persists
- [ ] Partial forms setting persists
- [ ] Timeout setting persists
- [ ] Journal expiration persists
- [ ] Update check setting persists
- [ ] Stat logging setting persists

## 9. Game-Specific Testing

### 9.1 Fallout 4
- [ ] FO4Edit.exe detected correctly
- [ ] FO4 skip list loaded
- [ ] Base game files skipped (Fallout4.esm, DLCs)
- [ ] DLC plugins handled correctly
- [ ] Creation Club content handled
- [ ] F4SE plugins detected (if applicable)

### 9.2 Skyrim Special Edition
- [ ] SSEEdit.exe detected correctly
- [ ] TES5Edit.exe works
- [ ] SSE skip list loaded
- [ ] Base game files skipped (Skyrim.esm, DLCs)
- [ ] Creation Club content handled
- [ ] SKSE plugins detected (if applicable)

### 9.3 Fallout New Vegas
- [ ] FNVEdit.exe detected correctly
- [ ] FNV skip list loaded
- [ ] Base game files skipped (FalloutNV.esm, DLCs)
- [ ] DLC plugins handled correctly

### 9.4 Fallout 3
- [ ] FO3Edit.exe detected correctly
- [ ] FO3 skip list loaded
- [ ] Base game files skipped (Fallout3.esm, DLCs)
- [ ] DLC plugins handled correctly

### 9.5 Tale of Two Wastelands
- [ ] TTWEdit.exe detected correctly
- [ ] TTW skip list loaded
- [ ] Combined FO3/FNV handling
- [ ] TTW-specific plugins handled

## 10. Performance Testing

### 10.1 Small Load Orders (1-50 plugins)
- [ ] Load time acceptable (<2s)
- [ ] Cleaning completes in reasonable time
- [ ] UI responsive throughout
- [ ] Memory usage reasonable (<100MB)

### 10.2 Medium Load Orders (50-200 plugins)
- [ ] Load time acceptable (<5s)
- [ ] Batch cleaning efficient
- [ ] UI remains responsive
- [ ] Memory usage reasonable (<200MB)

### 10.3 Large Load Orders (200+ plugins)
- [ ] Load time acceptable (<10s)
- [ ] Can handle full load order
- [ ] Concurrent cleaning limited appropriately
- [ ] Memory usage acceptable (<500MB)
- [ ] No performance degradation over time

### 10.4 Long-Running Operations
- [ ] Cleaning 50+ plugins consecutively
- [ ] No memory leaks
- [ ] No performance degradation
- [ ] UI remains responsive
- [ ] Can cancel at any point

## 11. Regression Testing

After any code changes, verify these critical paths:

### 11.1 Happy Path
- [ ] Configure all paths
- [ ] Select plugins
- [ ] Start cleaning
- [ ] View results
- [ ] Repeat cleaning session

### 11.2 Configuration Changes
- [ ] Change xEdit executable
- [ ] Change game type
- [ ] Change load order file
- [ ] Add/remove MO2 path
- [ ] Toggle partial forms
- [ ] Change timeout value

### 11.3 Error Recovery
- [ ] Handle missing files
- [ ] Handle process failures
- [ ] Handle invalid configurations
- [ ] Recover from cancelled operations
- [ ] Restart after errors

## 12. Platform-Specific Testing (Windows)

### 12.1 Windows 10
- [ ] Application runs correctly
- [ ] File paths handled correctly
- [ ] Process execution works
- [ ] YAML files created/read properly

### 12.2 Windows 11
- [ ] Application runs correctly
- [ ] UI renders properly with Windows 11 theme
- [ ] File pickers work correctly
- [ ] Process execution works

### 12.3 Windows Paths
- [ ] C:\ drive paths work
- [ ] Other drive letters work (D:\, E:\, etc.)
- [ ] Network paths work (\\\\server\\share)
- [ ] Paths with spaces handled
- [ ] Long paths handled (>260 characters)

## 13. Accessibility Testing (Optional)

- [ ] Screen reader compatibility
- [ ] Keyboard-only navigation
- [ ] High contrast mode support
- [ ] Font scaling support
- [ ] Color blind friendly indicators

## 14. Documentation Testing

- [ ] README.md accurate and up-to-date
- [ ] FEATURE_PARITY.md reflects current state
- [ ] MANUAL_TESTING.md (this document) complete
- [ ] Code comments helpful
- [ ] Configuration examples valid
- [ ] Installation instructions work

## Test Results Template

Use this template to record test results:

```markdown
## Test Session: [Date]

**Tester**: [Name]
**Version**: [Version Number]
**Platform**: [OS Version]
**Game Tested**: [Game Name]

### Passed Tests
- [List of passed test sections]

### Failed Tests
- [Test Name]: [Description of failure]
- [Expected]: [What should happen]
- [Actual]: [What actually happened]
- [Steps to Reproduce]: [How to trigger the failure]

### Bugs Found
1. **[Bug Title]**
   - Severity: [Critical/High/Medium/Low]
   - Description: [Details]
   - Steps to Reproduce: [Steps]
   - Expected: [Expected behavior]
   - Actual: [Actual behavior]

### Notes
- [Any additional observations]
- [Performance notes]
- [Suggestions for improvement]
```

## Testing Priorities

If time is limited, prioritize testing in this order:

1. **Critical Path** (Must Test):
   - Configuration workflow
   - Basic single plugin cleaning
   - Results display
   - Error handling for missing files

2. **High Priority** (Should Test):
   - Batch cleaning (5-10 plugins)
   - Skip list integration
   - MO2 integration
   - Configuration persistence

3. **Medium Priority** (Nice to Test):
   - All game variants
   - Partial forms mode
   - Edge cases (special characters, long paths)
   - Performance with large load orders

4. **Low Priority** (Optional):
   - Accessibility features
   - UI/UX polish
   - Uncommon edge cases
   - Platform-specific quirks

## Automated vs Manual Testing

**Automated** (covered by integration tests):
- Command building logic
- Configuration loading/saving
- Game detection
- State management
- Statistics aggregation

**Manual** (requires human testing):
- UI rendering and layout
- User workflows and UX
- File picker dialogs
- xEdit subprocess execution
- Cross-platform behavior
- Error message clarity
- Performance under load

---

## Sign-Off

When all critical and high priority tests pass, the release is ready for deployment.

**Release Version**: _____________

**Testing Complete**: [ ]

**Tested By**: _____________

**Date**: _____________

**Approved By**: _____________

**Date**: _____________
