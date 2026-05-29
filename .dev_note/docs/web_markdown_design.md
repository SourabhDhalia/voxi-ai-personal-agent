# Design Document — Web Markdown Rendering Support

This document details the architectural and implementation design for rendering Markdown content natively on the Voxi web dashboard.

## 1. Architectural Architecture & Offline Support
Voxi runs in constrained, often offline environments (Voxi OS / Embedded Linux). Therefore:
- External CDN script loading is prohibited for core operations.
- `marked.min.js` will be stored locally inside the `data/web/` directory.
- Axum will serve this file statically along with other dashboard files.
- The web browser client will load the script from the local origin `/marked.min.js`.

## 2. Document Object Model (DOM) Structure
To render markdown successfully, raw `<pre>` tags must be replaced with `<div>` elements that can contain parsed HTML markup.

### Sessions Detail Page
- **Old Element**: `<pre class="viewer-content" id="session-viewer-content"></pre>`
- **New Element**: `<div class="viewer-content markdown-body" id="session-viewer-content"></div>`

### Tasks Detail Page
- **Old Element**: `<pre class="viewer-content" id="task-viewer-content"></pre>`
- **New Element**: `<div class="viewer-content markdown-body" id="task-viewer-content"></div>`

## 3. JavaScript Event & Parsing Logic (`app.js`)

### Chat Feed Rendering
Modify `addChatMsg(role, text)`:
```javascript
function addChatMsg(role, text) {
    if (!chatMessages) return;
    const welcome = chatMessages.querySelector('.chat-welcome');
    if (welcome) welcome.remove();

    const el = document.createElement('div');
    el.className = 'chat-msg ' + role;
    if (role === 'assistant' && typeof marked !== 'undefined' && typeof marked.parse === 'function') {
        el.innerHTML = marked.parse(text);
    } else {
        el.textContent = text;
    }
    chatMessages.appendChild(el);
    chatMessages.scrollTop = chatMessages.scrollHeight;
}
```

### Session Details & Task Details Rendering
Set `.innerHTML` using `marked.parse` if `marked` is loaded, falling back to escaped text content if missing.
```javascript
content.innerHTML = (typeof marked !== 'undefined' && typeof marked.parse === 'function')
    ? marked.parse(resp.content)
    : escHtml(resp.content);
```

## 4. UI/UX Style Adaptation (`style.css`)
To keep the dashboard aesthetic:
- The class `.markdown-body` will define standard typography constraints.
- Lists (`ul`, `ol`) will have bullet markers and indentation restored (the general styles reset list markers for the navbar).
- Code blocks (`pre`, `code`) will have a clean dark background, code highlights, and use `JetBrains Mono` with proper padding and borders.
- Tables will have borders, striped row backgrounds, and padding.
- Headings (`h1`, `h2`, `h3`) will have adjusted margins, borders, and weights to align with the dashboard's design system.
