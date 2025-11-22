# ESC Key Behavior in ARULA CLI

## Overview

The ESC key in ARULA now has smart dual functionality based on context and timing.

---

## 🎯 ESC Key Behavior

### **Single ESC Press**

**When AI is responding:**
- ✅ **Cancels the ongoing AI request**
- Stops the spinner
- Shows message: `🛑 Request cancelled (ESC pressed)`
- Returns to ready state

**When idle (no AI request):**
- ✅ **Clears current input**
- Empties the input buffer
- Resets to blank prompt

### **Double ESC Press** (within 1 second)

**Always:**
- ✅ **Opens the main menu**
- Same as typing `m`, `menu`, or `/menu`
- Shows ARULA's overlay menu system

---

## 🔧 Technical Implementation

### State Tracking
```rust
pub struct AppState {
    pub esc_count: usize,              // Number of ESC presses
    pub last_esc_time: Instant,        // Timestamp of last ESC
}
```

### Logic Flow
```
ESC Pressed
    ↓
Track timestamp & increment counter
    ↓
Check elapsed time since last ESC
    ↓
If > 1 second → Reset counter to 1
    ↓
If counter == 1 → Single ESC behavior
If counter >= 2 → Double ESC behavior (show menu)
```

### Return Values
```rust
// Special strings returned by read_line()
"__ESC__"        // Single ESC pressed
"__SHOW_MENU__"  // Double ESC pressed
```

---

## 📊 Behavior Matrix

| Context | ESC Count | Action | Message |
|---------|-----------|--------|---------|
| AI responding | 1 | Cancel request | 🛑 Request cancelled |
| Idle | 1 | Clear input | (silent) |
| Any | 2 (within 1s) | Show menu | (menu appears) |
| Any | ESC after >1s | Reset counter | (treated as first ESC) |

---

## 💡 Usage Examples

### Example 1: Cancel AI Request
```
⚡[50] s:a2f3 > Tell me about quantum computing

AI starts responding...
🔧 [Thinking...]

[User presses ESC]

🛑 Request cancelled (ESC pressed)
⚡[50] s:a2f3 >
```

### Example 2: Clear Input
```
⚡[50] s:a2f3 > This is a mistake I want to cle█

[User presses ESC]

⚡[50] s:a2f3 > █
# Input cleared
```

### Example 3: Open Menu
```
⚡[50] s:a2f3 > █

[User presses ESC twice quickly]

╭─ ARULA Menu ─╮
│ ▶ 💬 Continue Chat     │
│   ⚙️  Settings          │
│   ℹ️  Info & Help       │
│   🧹 Clear Chat         │
│   🚪 Exit ARULA         │
╰───────────────────────╯
```

### Example 4: ESC Timeout
```
⚡[50] s:a2f3 > █

[User presses ESC]
# Input cleared

[User waits 2 seconds]

⚡[50] s:a2f3 > Hello█

[User presses ESC]
# Input cleared (counter was reset due to timeout)
```

---

## 🎨 Alternative Menu Access

While ESC is now the fastest way, you still have these options:

1. **Type `m`** and press Enter
2. **Type `menu`** and press Enter
3. **Type `/menu`** and press Enter

All four methods work identically!

---

## 🔧 Configuration

### Timeout Duration
The ESC timeout is hardcoded to **1 second**. To customize:

Edit `src/reedline_input.rs`:
```rust
// Line ~395
if elapsed.as_secs() > 1 {  // Change this value
    state.esc_count = 0;
}
```

### Custom ESC Behavior
To modify what single ESC does, edit `src/main.rs`:
```rust
if input == "__ESC__" {
    // Your custom logic here
    if app.is_waiting_for_response() {
        // Cancel request
        app.cancel_request();
    }
    // Add other behaviors...
}
```

---

## 🐛 Edge Cases

### What if I press ESC 3 times?
- First ESC: Clear/Cancel
- Second ESC: Show menu
- Third ESC: *While menu is open* - closes menu, returns to input

### What happens during menu navigation?
- ESC in menu closes the menu
- Returns to normal input mode
- ESC counter resets

### Can I accidentally trigger menu while canceling?
- No! The 1-second timeout prevents this
- You'd need to press ESC twice within 1 second
- Normal usage won't accidentally open menu

---

## 📝 Implementation Notes

### Why Ctrl+C triggers ESC logic?
Reedline maps ESC key to `Signal::CtrlC` internally. We intercept this signal and add our own logic before reedline processes it.

### Why not use native reedline menu?
The current implementation uses ARULA's existing overlay menu system for consistency. Future versions may migrate to reedline's native menu system.

### Thread Safety
ESC state is protected by `Arc<Mutex<AppState>>`, making it safe for concurrent access across the async event loop.

---

## 🚀 Future Enhancements

Potential improvements for future versions:

1. **Configurable timeout** - User-adjustable ESC timing
2. **Triple-ESC action** - Additional quick access feature
3. **Visual feedback** - Show "Press ESC again for menu" hint
4. **Haptic feedback** - Terminal bell on double-ESC
5. **Menu preview** - Show small menu hint on first ESC

---

**Feature Status**: ✅ **Implemented and Tested**
**Version**: Added in reedline migration (2025-01-22)
**Maintainer**: ARULA CLI Team
