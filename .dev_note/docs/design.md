# UI/UX Redesign Specification: Voxi Liquid Glass Theme

## 1. Design Tokens & CSS Variables

We will utilize and extend the existing `:root` variables in `data/web/style.css` to define the Apple-inspired Liquid Glass theme.

### Color Tokens
- **Accent (Apple Blue)**: `--accent: #007aff` (glow: `rgba(0,122,255,0.15)`)
- **Accent Teal**: `#64ffda` (glow: `rgba(100,255,218,0.15)`)
- **Accent Green**: `#34c759`
- **Accent Orange**: `#ff9500`
- **Accent Purple**: `#af52de`
- **Base Background (OLED Dark)**: `--bg-primary: #000000`
- **Card Background (Translucent Glass)**: `--bg-glass: rgba(255, 255, 255, 0.05)`
- **Secondary Background**: `--bg-secondary: #0a0a0c`
- **Glass Borders**: `--border-glass: rgba(255, 255, 255, 0.08)`

### Glassmorphism System
- **Filter**: `backdrop-filter: blur(20px) saturate(190%)`
- **Box Shadow**: `0 8px 32px 0 rgba(0, 0, 0, 0.37)`
- **Border**: `1px solid var(--border-glass)`
- **Border Radius Scale**:
  - Hero Card: `24px`
  - Compact Tile: `16px`
  - Pill highlight / Input bar: `9999px` (capsule)

---

## 2. Page Redesigns

### R1. Dashboard Page (`#page-dashboard`)
We will replace the uniform grid layouts with a Bento-style asymmetric grid layout.

#### Bento Grid Architecture
We will introduce a CSS grid structure `.bento-dashboard-grid` inside `#page-dashboard`.

- **Grid Sizing**: 12-column layout on desktop, responsive breakdown to 1-column on mobile.
- **Asymmetric Spanning**:
  1. **Agent Status Card** (Hero): Span 8 columns on desktop, 2 rows. Includes live status text `#stat-status` and dynamic status-colored icons.
  2. **System Health Stats (Memory, CPU, Threads, PID)**: Arranged in a compact sub-grid within 4 columns.
  3. **Uptime & Sessions**: Spans 4 columns.
  4. **Live Trends (CPU, Memory, Calls, Tokens)**:
     - CPU and Memory sparkline cards: Span 6 columns each (horizontal pair).
     - Calls and Tokens sparkline cards: Span 6 columns each.
  5. **Model Providers**: Redesigned as dynamic chip/pill cards flex-wrapped instead of list items, styled with translucent frosted backgrounds.

### R2. Sidebar Navigation
- **Background**: Translucent glass (`background: rgba(10, 10, 12, 0.65)`, `backdrop-filter: blur(20px)`).
- **Navigation Active Highlighting**: Pill highlight shape around the active nav list item (`background: rgba(255, 255, 255, 0.08)` or accent overlay, `border-radius: 9999px`).
- **Logo Area**: Refined header with a thin horizontal linear gradient divider (`background: linear-gradient(90deg, transparent, var(--accent), transparent)`).
- **Footer**: Status label and indicator dot nested in a compact pill container.

### R3. Chat Page (`#page-chat`)
- **Bubbles**: Frosted glass material (`backdrop-filter: blur(10px)`) with rounded corners of `20px+`.
  - **User Messages**: Slanted shape (e.g. `border-radius: 20px 20px 4px 20px`), right-aligned, light grey/blue tinted glass (`background: rgba(0, 122, 255, 0.15)`).
  - **Assistant Messages**: Slanted shape (e.g. `border-radius: 20px 20px 20px 4px`), left-aligned, dark/subtle tinted glass (`background: rgba(255, 255, 255, 0.05)`).
- **Input Bar**: Floating container (`position: sticky` or absolute inside chat workspace) with full capsule rounded corners (`border-radius: 9999px`), centered, containing the input field and actions inside a unified glass container.

### R4. Global Visual Polish
- **Typography hierarchy**: Section labels styled in small uppercase with letter-spacing (`letter-spacing: 0.05em`). Hero values set to `2rem` bold.
- **Scrollbars**: Styled to be thin and translucent (`scrollbar-width: thin`, `::-webkit-scrollbar` with translucent thumb).
- **Empty States**: Renders centered Lucide-style SVGs and dimmed explanatory labels.
- **Micro-animations**:
  - Hover on cards and nav items: `transform: translateY(-2px) scale(1.01)` with transition `0.25s cubic-bezier(0.16, 1, 0.3, 1)`.
  - Button presses: `transform: scale(0.97)` on active state.

---

## 3. Preservation and Integration Checklist
- [x] All DOM IDs (e.g. `stat-status`, `stat-uptime`, `stat-memory`, `chart-cpu`, `chat-messages`, etc.) must remain unchanged.
- [x] Class actions, specifically `.active` for pages and navigation items, must be kept intact.
- [x] The customizer toggles `#customizer-toggle` and `#customizer-modal` variables in CSS and logic are preserved.
