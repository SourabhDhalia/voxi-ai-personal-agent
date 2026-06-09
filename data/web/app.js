// Voxi Dashboard — Vanilla JS SPA
(function () {
    'use strict';

    const API = '';  // Same origin

    // --- Auth State ---
    let authToken =
        localStorage.getItem('admin_token') ||
        sessionStorage.getItem('admin_token');
    let eventSource = null;
    let approvalCountdownTimer = null;

    function persistAdminToken(token) {
        authToken = token;
        localStorage.setItem('admin_token', token);
        sessionStorage.setItem('admin_token', token);
        initEventStream();
    }

    function clearAdminToken() {
        authToken = null;
        localStorage.removeItem('admin_token');
        sessionStorage.removeItem('admin_token');
        if (eventSource) {
            eventSource.close();
            eventSource = null;
        }
    }

    function getAuthHeaders() {
        return authToken
            ? { 'Authorization': 'Bearer ' + authToken }
            : {};
    }

    // --- Navigation ---
    const navItems =
        document.querySelectorAll('.nav-item');
    const pages =
        document.querySelectorAll('.page');
    let metricsInterval = null;
    let outboundPollTimer = null;
    let outboundCursor =
        parseInt(localStorage.getItem(
            'dashboard_outbound_cursor') || '0', 10) || 0;

    function navigateTo(page) {
        navItems.forEach(n =>
            n.classList.remove('active'));
        pages.forEach(p =>
            p.classList.remove('active'));

        // Stop dashboard auto-refresh when leaving
        if (page !== 'dashboard' && metricsInterval) {
            clearInterval(metricsInterval);
            metricsInterval = null;
        }

        if (page !== 'chat' && window.chatPollInterval) {
            clearInterval(window.chatPollInterval);
            window.chatPollInterval = null;
        }

        const navEl =
            document.getElementById('nav-' + page);
        const pageEl =
            document.getElementById('page-' + page);
        if (navEl) navEl.classList.add('active');
        if (pageEl) pageEl.classList.add('active');

        if (page === 'dashboard') loadDashboard();
        else if (page === 'search') focusSearch();
        else if (page === 'sessions') loadSessions();
        else if (page === 'tasks') loadTasks();
        else if (page === 'logs') loadLogs();
        else if (page === 'chat') {
            loadChatSessions().then(() => {
                if (currentChatSessionId) {
                    loadChatSessionDetail(currentChatSessionId);
                }
            });
        }
        else if (page === 'skills') loadSkills();
        else if (page === 'ota') loadOta();
        else if (page === 'admin') loadAdmin();
    }

    navItems.forEach(item => {
        item.addEventListener('click', () => {
            navigateTo(item.dataset.page);
        });
    });

    // --- API Helpers ---
    async function apiFetch(endpoint, opts) {
        try {
            const headers = Object.assign(
                {}, getAuthHeaders(),
                (opts && opts.headers) || {});
            const res = await fetch(
                API + '/api/' + endpoint,
                Object.assign({}, opts, { headers }));
            const data = await res.json();
            data.__http_status = res.status;
            if (res.status === 401) {
                handleAdminUnauthorized(
                    data.error || 'Session expired');
            }
            return data;
        } catch (e) {
            console.error('API error:', e);
            return null;
        }
    }

    async function apiPost(endpoint, body) {
        return apiFetch(endpoint, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(body)
        });
    }

    async function apiDelete(endpoint, body) {
        const opts = { method: 'DELETE' };
        if (body !== undefined) {
            opts.headers = {
                'Content-Type': 'application/json'
            };
            opts.body = JSON.stringify(body);
        }
        return apiFetch(endpoint, opts);
    }

    function persistOutboundCursor(cursor) {
        outboundCursor = cursor || 0;
        localStorage.setItem(
            'dashboard_outbound_cursor',
            String(outboundCursor));
    }

    async function pollOutboundMessages() {
        const resp = await apiFetch(
            'outbound/messages?since=' + outboundCursor);
        if (!resp || !Array.isArray(resp.messages)) {
            return;
        }
        resp.messages.forEach(msg => {
            const text = msg.title
                ? (msg.title + ': ' + msg.message)
                : msg.message;
            showToast(text, 'success', 5000);
        });
        if (typeof resp.cursor === 'number' &&
            resp.cursor >= outboundCursor) {
            persistOutboundCursor(resp.cursor);
        }
    }

    function startOutboundPolling() {
        if (outboundPollTimer) return;
        pollOutboundMessages();
        outboundPollTimer = setInterval(
            pollOutboundMessages, 8000);
    }

    // --- Date Breadcrumb Navigator ---
    const MONTHS = [
        'Jan', 'Feb', 'Mar', 'Apr',
        'May', 'Jun', 'Jul', 'Aug',
        'Sep', 'Oct', 'Nov', 'Dec'
    ];

    class DateNav {
        constructor(elementId, onSelect) {
            this.el = document.getElementById(
                elementId);
            this.onSelect = onSelect;
            this.dates = [];
            this.level = 'year';
            this.selYear = null;
            this.selMonth = null;
            this.selDay = null;
        }

        setDates(dateStrings) {
            this.dates = (dateStrings || [])
                .slice().sort().reverse();
            this.level = 'year';
            this.selYear = null;
            this.selMonth = null;
            this.selDay = null;
            this.render();
        }

        getYears() {
            const s = new Set();
            this.dates.forEach(d =>
                s.add(d.substring(0, 4)));
            return [...s].sort().reverse();
        }

        getMonths(year) {
            const s = new Set();
            this.dates.forEach(d => {
                if (d.substring(0, 4) === year)
                    s.add(d.substring(5, 7));
            });
            return [...s].sort().reverse();
        }

        getDays(year, month) {
            return this.dates
                .filter(d =>
                    d.substring(0, 4) === year &&
                    d.substring(5, 7) === month)
                .map(d => d.substring(8, 10))
                .sort().reverse();
        }

        render() {
            if (!this.el) return;
            let html =
                '<span class="date-nav-label">' +
                '📅 Browse</span>';

            if (this.level === 'year') {
                const years = this.getYears();
                if (years.length === 0) {
                    html += '<span class=' +
                        '"date-nav-chip" ' +
                        'style="cursor:default;' +
                        'opacity:0.5">' +
                        'No dates</span>';
                } else {
                    years.forEach(y => {
                        html +=
                            '<span class=' +
                            '"date-nav-chip" ' +
                            'data-year="' +
                            y + '">' +
                            y + '</span>';
                    });
                }
            } else if (this.level === 'month') {
                html +=
                    '<span class="date-nav-chip' +
                    ' breadcrumb" data-reset=' +
                    '"year">' +
                    'All Years</span>' +
                    '<span class="date-nav-sep"' +
                    '>›</span>' +
                    '<span class="date-nav-chip' +
                    ' breadcrumb-current">' +
                    this.selYear + '</span>' +
                    '<span class="date-nav-sep"' +
                    '>›</span>';
                const months =
                    this.getMonths(this.selYear);
                months.forEach(m => {
                    const mi = parseInt(m, 10) - 1;
                    html +=
                        '<span class=' +
                        '"date-nav-chip" ' +
                        'data-month="' + m +
                        '">' +
                        MONTHS[mi] +
                        '</span>';
                });
            } else if (this.level === 'day') {
                html +=
                    '<span class="date-nav-chip' +
                    ' breadcrumb" data-reset=' +
                    '"year">' +
                    'All Years</span>' +
                    '<span class="date-nav-sep"' +
                    '>›</span>' +
                    '<span class="date-nav-chip' +
                    ' breadcrumb" data-reset=' +
                    '"month">' +
                    this.selYear + '</span>' +
                    '<span class="date-nav-sep"' +
                    '>›</span>' +
                    '<span class="date-nav-chip' +
                    ' breadcrumb-current">' +
                    MONTHS[
                        parseInt(this.selMonth,
                            10) - 1] +
                    '</span>' +
                    '<span class="date-nav-sep"' +
                    '>›</span>';
                const days = this.getDays(
                    this.selYear, this.selMonth);
                days.forEach(d => {
                    const full =
                        this.selYear + '-' +
                        this.selMonth + '-' + d;
                    const cls =
                        (this.selDay === d)
                            ? 'date-nav-chip active'
                            : 'date-nav-chip';
                    html +=
                        '<span class="' + cls +
                        '" data-day="' + d +
                        '" data-full="' +
                        full + '">' +
                        parseInt(d, 10) +
                        '</span>';
                });
            }

            // Show All button when filtered
            if (this.selYear || this.selDay) {
                html += '<span class=' +
                    '"date-nav-all" ' +
                    'data-reset="all">' +
                    '✕ Show All</span>';
            }

            this.el.innerHTML = html;
            this.bind();
        }

        bind() {
            if (!this.el) return;
            const self = this;
            this.el.querySelectorAll(
                '[data-year]').forEach(el => {
                    el.addEventListener(
                        'click', () => {
                            self.selYear =
                                el.dataset.year;
                            self.level = 'month';
                            self.render();
                        });
                });
            this.el.querySelectorAll(
                '[data-month]').forEach(el => {
                    el.addEventListener(
                        'click', () => {
                            self.selMonth =
                                el.dataset.month;
                            self.level = 'day';
                            self.selDay = null;
                            self.render();
                            self.onSelect(null);
                        });
                });
            this.el.querySelectorAll(
                '[data-day]').forEach(el => {
                    el.addEventListener(
                        'click', () => {
                            self.selDay =
                                el.dataset.day;
                            self.render();
                            self.onSelect(
                                el.dataset.full);
                        });
                });
            this.el.querySelectorAll(
                '[data-reset]').forEach(el => {
                    el.addEventListener(
                        'click', () => {
                            const r =
                                el.dataset.reset;
                            if (r === 'year' ||
                                r === 'all') {
                                self.selYear = null;
                                self.selMonth = null;
                                self.selDay = null;
                                self.level = 'year';
                            } else if (
                                r === 'month') {
                                self.selMonth = null;
                                self.selDay = null;
                                self.level =
                                    'month';
                            }
                            self.render();
                            if (r === 'all')
                                self.onSelect(null);
                        });
                });
        }

        getFilter() {
            if (!this.selYear) return null;
            if (!this.selMonth)
                return this.selYear;
            if (!this.selDay)
                return this.selYear + '-' +
                    this.selMonth;
            return this.selYear + '-' +
                this.selMonth + '-' +
                this.selDay;
        }

        selectDate(dateStr) {
            if (!dateStr || dateStr.length !== 10) return;
            this.selYear = dateStr.substring(0, 4);
            this.selMonth = dateStr.substring(5, 7);
            this.selDay = dateStr.substring(8, 10);
            this.level = 'day';
            this.render();
        }
    }

    // --- Dashboard ---
    async function loadDashboard() {
        if (metricsInterval)
            clearInterval(metricsInterval);
        await refreshMetrics();
        loadLlmConfig();
        metricsInterval = setInterval(refreshMetrics, 5000);
    }

    async function loadLlmConfig() {
        const el = document.getElementById('llm-providers');
        if (!el) return;
        const cfg = await apiFetch('llm/config');
        if (!cfg || typeof cfg !== 'object') {
            el.innerHTML = '<p class="empty-state">Model configuration unavailable.</p>';
            return;
        }
        const active = cfg.active_backend || '—';
        const fallbacks = Array.isArray(cfg.fallback_backends) ? cfg.fallback_backends : [];
        const providers = (cfg.providers && typeof cfg.providers === 'object')
            ? Object.keys(cfg.providers) : [];
        let html = '<div class="llm-card">';
        html += '<div class="llm-row"><span class="llm-label">Active backend</span>'
            + '<span class="llm-active">' + escHtml(String(active)) + '</span></div>';
        if (fallbacks.length) {
            html += '<div class="llm-row"><span class="llm-label">Fallbacks</span>'
                + '<span class="llm-value">' + fallbacks.map(f => escHtml(String(f))).join(', ') + '</span></div>';
        }
        if (providers.length) {
            html += '<div class="llm-row"><span class="llm-label">Configured</span>'
                + '<span class="llm-value">' + providers.map(p => escHtml(String(p))).join(', ') + '</span></div>';
        }
        html += '</div>';
        el.innerHTML = html;
    }

    function fmtTokens(n) {
        if (n == null) return '—';
        if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
        if (n >= 1000)    return (n / 1000).toFixed(1) + 'K';
        return String(n);
    }

    async function refreshMetrics() {
        const [m, sessions, tasks] = await Promise.all([
            apiFetch('metrics'),
            apiFetch('sessions'),
            apiFetch('tasks'),
        ]);

        const s = id => document.getElementById(id);

        if (m) {
            const connected = m.agent_connected === true;

            // Sidebar agent status indicator
            const dot = s('status-dot');
            const label = s('status-label');
            if (dot)   dot.className = 'status-dot ' + (connected ? 'running' : 'disconnected');
            if (label) label.textContent = connected ? 'Agent running' : 'Agent offline';

            // LLM usage section badge
            const badge = s('usage-source-badge');
            if (badge) {
                badge.textContent = connected ? 'live' : 'offline';
                badge.style.background = connected
                    ? 'rgba(5,150,105,0.1)' : 'rgba(220,38,38,0.08)';
                badge.style.color = connected ? 'var(--success)' : 'var(--danger)';
                badge.style.borderColor = connected
                    ? 'rgba(5,150,105,0.15)' : 'rgba(220,38,38,0.15)';
            }

            // Primary stats
            if (s('stat-status')) {
                const txt = m.status || '—';
                s('stat-status').textContent = txt;
                const icon = s('stat-status').closest('.stat-card')
                    && s('stat-status').closest('.stat-card').querySelector('.stat-icon');
                if (icon) {
                    icon.className = 'stat-icon ' +
                        (txt === 'running' ? 'stat-icon--green' :
                         txt === 'disconnected' ? 'stat-icon--red' : 'stat-icon--amber');
                }
            }
            s('stat-uptime') &&
                (s('stat-uptime').textContent =
                    (m.uptime && m.uptime.formatted) || '—');

            // System health
            s('stat-memory') &&
                (s('stat-memory').textContent =
                    (m.memory && m.memory.vm_rss_kb)
                        ? (m.memory.vm_rss_kb / 1024).toFixed(1) + ' MB' : '—');
            s('stat-cpu') &&
                (s('stat-cpu').textContent =
                    (m.cpu && m.cpu.load_1m != null)
                        ? m.cpu.load_1m.toFixed(2) : '—');
            s('stat-threads') && (s('stat-threads').textContent = m.threads || '—');
            s('stat-pid')     && (s('stat-pid').textContent = m.pid || '—');

            // LLM counters
            s('stat-errors') &&
                (s('stat-errors').textContent =
                    (m.counters && m.counters.errors != null)
                        ? m.counters.errors : '—');
            s('stat-llm') &&
                (s('stat-llm').textContent =
                    (m.counters && m.counters.llm_calls != null)
                        ? m.counters.llm_calls : (connected ? '0' : '—'));
            s('stat-tools') &&
                (s('stat-tools').textContent =
                    (m.counters && m.counters.tool_calls != null)
                        ? m.counters.tool_calls : '—');

            // Token usage (new)
            if (m.tokens) {
                s('stat-prompt-tokens')     && (s('stat-prompt-tokens').textContent     = fmtTokens(m.tokens.prompt));
                s('stat-completion-tokens') && (s('stat-completion-tokens').textContent = fmtTokens(m.tokens.completion));
                s('stat-cache-read')        && (s('stat-cache-read').textContent        = fmtTokens(m.tokens.cache_read));
                s('stat-cache-write')       && (s('stat-cache-write').textContent       = fmtTokens(m.tokens.cache_write));
            } else {
                ['stat-prompt-tokens','stat-completion-tokens',
                 'stat-cache-read','stat-cache-write'].forEach(id => {
                    s(id) && (s(id).textContent = connected ? '0' : '—');
                });
            }

            updateCharts(m);
        }

        if (sessions && s('stat-sessions'))
            s('stat-sessions').textContent = sessions.length;
        if (tasks && s('stat-tasks'))
            s('stat-tasks').textContent = tasks.length;
    }

    // --- Sessions ---
    let sessionDateNav = null;
    let allSessions = null;

    async function loadSessions(filterDate) {
        const data = await apiFetch('sessions');
        const list =
            document.getElementById('session-list');
        const viewer =
            document.getElementById('session-viewer');
        viewer.style.display = 'none';
        list.style.display = '';

        allSessions = data || [];

        // Init date nav once
        if (!sessionDateNav) {
            sessionDateNav = new DateNav(
                'session-date-nav',
                function (date) {
                    renderSessions(date);
                });
        }
        // Collect unique dates
        const dates = allSessions.map(
            s => s.date).filter(Boolean);
        sessionDateNav.setDates(
            [...new Set(dates)]);

        renderSessions(filterDate || null);
    }

    function renderSessions(filterDate) {
        const list =
            document.getElementById('session-list');
        let items = allSessions || [];

        if (filterDate) {
            items = items.filter(
                s => s.date === filterDate);
        } else {
            // If nav has partial filter
            // (year or year-month)
            const f = sessionDateNav
                ? sessionDateNav.getFilter()
                : null;
            if (f) {
                items = items.filter(
                    s => s.date &&
                        s.date.startsWith(f));
            }
        }

        if (items.length === 0) {
            list.innerHTML =
                '<p class="empty-state">' +
                'No sessions found</p>';
            return;
        }

        list.innerHTML = items.map(s => {
            const sizeKB =
                (s.size_bytes / 1024).toFixed(1);
            const modified = s.modified ?
                new Date(s.modified * 1000)
                    .toLocaleString() : '—';
            return '<div class="card-item ' +
                'clickable" data-session-id="' +
                escHtml(s.id) + '">' +
                '<div class="card-item-title">' +
                escHtml(s.title || s.id) + '</div>' +
                '<div class="card-item-meta">' +
                escHtml(s.id) + ' · ' +
                sizeKB + ' KB · ' +
                modified + '</div></div>';
        }).join('');

        list.querySelectorAll('.card-item')
            .forEach(card => {
                card.addEventListener(
                    'click', () => {
                        showSessionDetail(
                            card.dataset
                                .sessionId);
                    });
            });
    }

    async function showSessionDetail(id) {
        const list =
            document.getElementById('session-list');
        const viewer =
            document.getElementById('session-viewer');
        const title = document.getElementById(
            'session-viewer-title');
        const content = document.getElementById(
            'session-viewer-content');

        list.style.display = 'none';
        viewer.style.display = '';
        title.textContent = id;
        content.textContent = 'Loading...';

        const resp =
            await apiFetch('sessions/' + id);
        if (resp && resp.content) {
            content.innerHTML = (typeof marked !== 'undefined' && typeof marked.parse === 'function')
                ? marked.parse(resp.content)
                : escHtml(resp.content);
        } else {
            content.innerHTML =
                'Failed to load session.';
        }
    }

    document.getElementById('session-back')
        .addEventListener('click', () => {
            document.getElementById('session-viewer')
                .style.display = 'none';
            document.getElementById('session-list')
                .style.display = '';
        });

    // --- Tasks ---
    let taskDateNav = null;
    let allTasks = null;
    let currentTaskFile = null;
    let selectedTaskIds = new Set();

    const taskSelectAllBtn =
        document.getElementById('task-select-all');
    const taskDeleteSelectedBtn =
        document.getElementById('task-delete-selected');
    const taskSelectionMeta =
        document.getElementById('task-selection-meta');
    const taskDeleteCurrentBtn =
        document.getElementById('task-delete-current');

    async function loadTasks(filterDate) {
        const data = await apiFetch('tasks');
        const list =
            document.getElementById('task-list');
        const viewer =
            document.getElementById('task-viewer');
        viewer.style.display = 'none';
        list.style.display = '';
        currentTaskFile = null;

        allTasks = data || [];
        const availableTaskIds = new Set(
            allTasks.map(t => t.id).filter(Boolean));
        selectedTaskIds.forEach(id => {
            if (!availableTaskIds.has(id)) {
                selectedTaskIds.delete(id);
            }
        });

        // Init date nav once
        if (!taskDateNav) {
            taskDateNav = new DateNav(
                'task-date-nav',
                function (date) {
                    renderTasks(date);
                });
        }
        const dates = allTasks.map(
            t => t.date).filter(Boolean);
        taskDateNav.setDates(
            [...new Set(dates)]);

        renderTasks(filterDate || null);
        formatTaskSelectionMeta();
    }

    function renderTasks(filterDate) {
        const list =
            document.getElementById('task-list');
        let items = allTasks || [];

        if (filterDate) {
            items = items.filter(
                t => t.date === filterDate);
        } else {
            const f = taskDateNav
                ? taskDateNav.getFilter()
                : null;
            if (f) {
                items = items.filter(
                    t => t.date &&
                        t.date.startsWith(f));
            }
        }

        if (items.length === 0) {
            list.innerHTML =
                '<p class="empty-state">' +
                'No tasks found</p>';
            return;
        }

        list.innerHTML = items.map(t => {
            const modified = t.modified ?
                new Date(t.modified * 1000)
                    .toLocaleString() : '';
            const checked = selectedTaskIds
                .has(t.id) ? ' checked' : '';
            return '<div class="card-item task-card-item ' +
                'clickable" data-task-file="' +
                escHtml(t.file) + '" data-task-id="' +
                escHtml(t.id) + '">' +
                '<label class="chat-session-check">' +
                '<input type="checkbox" data-task-select="' +
                escHtml(t.id) + '"' + checked + ' />' +
                '<span></span></label>' +
                '<div class="card-item-title">' +
                escHtml(t.title || t.file) + '</div>' +
                '<div class="card-item-meta">' +
                (modified ? modified + ' · '
                    : '') +
                escHtml(t.file) + ' · ' +
                escHtml(
                    t.content_preview || '') +
                '</div></div>';
        }).join('');

        list.querySelectorAll('.card-item')
            .forEach(card => {
                card.addEventListener(
                    'click', (event) => {
                        if (event.target.closest(
                            '[data-task-select]')) {
                            return;
                        }
                        showTaskDetail(
                            card.dataset
                                .taskFile);
                    });
            });
        list.querySelectorAll('[data-task-select]')
            .forEach(input => {
                input.addEventListener('change', () => {
                    const id = input.dataset.taskSelect;
                    if (!id) return;
                    if (input.checked) {
                        selectedTaskIds.add(id);
                    } else {
                        selectedTaskIds.delete(id);
                    }
                    formatTaskSelectionMeta();
                });
            });
    }

    async function showTaskDetail(file) {
        const list =
            document.getElementById('task-list');
        const viewer =
            document.getElementById('task-viewer');
        const title = document.getElementById(
            'task-viewer-title');
        const content = document.getElementById(
            'task-viewer-content');

        list.style.display = 'none';
        viewer.style.display = '';
        title.textContent = file;
        content.textContent = 'Loading...';
        currentTaskFile = file;

        const resp =
            await apiFetch('tasks/' + file);
        if (resp && resp.content) {
            content.innerHTML = (typeof marked !== 'undefined' && typeof marked.parse === 'function')
                ? marked.parse(resp.content)
                : escHtml(resp.content);
        } else {
            content.innerHTML =
                'Failed to load task.';
        }
    }

    function formatTaskSelectionMeta() {
        if (!taskSelectionMeta) return;
        const count = selectedTaskIds.size;
        taskSelectionMeta.textContent = count
            ? ('Selected tasks: ' + count)
            : 'No tasks selected';
    }

    async function deleteTasks(ids) {
        const filteredIds = (ids || [])
            .filter(Boolean);
        if (!filteredIds.length) return;
        if (!window.confirm(
            'Delete selected tasks?')) {
            return;
        }

        let resp = null;
        if (filteredIds.length === 1) {
            resp = await apiDelete(
                'tasks/' +
                encodeURIComponent(filteredIds[0]));
        } else {
            resp = await apiDelete(
                'tasks', {
                    ids: filteredIds
                });
        }

        if (!resp || !resp.deleted_ids) {
            window.alert('Failed to delete tasks.');
            return;
        }

        resp.deleted_ids.forEach(id => {
            selectedTaskIds.delete(id);
            if (currentTaskFile === id ||
                currentTaskFile === id + '.md') {
                currentTaskFile = null;
                document.getElementById('task-viewer')
                    .style.display = 'none';
                document.getElementById('task-list')
                    .style.display = '';
            }
        });
        await loadTasks(taskDateNav
            ? taskDateNav.getFilter()
            : null);
    }

    document.getElementById('task-back')
        .addEventListener('click', () => {
            document.getElementById('task-viewer')
                .style.display = 'none';
            document.getElementById('task-list')
                .style.display = '';
        });
    if (taskSelectAllBtn) {
        taskSelectAllBtn.addEventListener(
            'click', () => {
                const taskIds = (allTasks || [])
                    .map(task => task.id)
                    .filter(Boolean);
                if (selectedTaskIds.size ===
                    taskIds.length) {
                    selectedTaskIds.clear();
                } else {
                    selectedTaskIds =
                        new Set(taskIds);
                }
                renderTasks(taskDateNav
                    ? taskDateNav.getFilter()
                    : null);
                formatTaskSelectionMeta();
            });
    }
    if (taskDeleteSelectedBtn) {
        taskDeleteSelectedBtn.addEventListener(
            'click', async () => {
                await deleteTasks(Array.from(
                    selectedTaskIds));
            });
    }
    if (taskDeleteCurrentBtn) {
        taskDeleteCurrentBtn.addEventListener(
            'click', async () => {
                if (!currentTaskFile) return;
                const taskId = currentTaskFile
                    .endsWith('.md')
                    ? currentTaskFile.slice(0, -3)
                    : currentTaskFile;
                await deleteTasks([taskId]);
            });
    }

    // --- Logs ---
    let logDateNav = null;
    let logDates = [];
    let currentLogDate = null;
    // Per-file pagination state: { [label]: { offset: 0, totalLines: 0 } }
    let logPagination = {};
    const LOG_PAGE_SIZE = 500;

    function formatDateHuman(dateStr) {
        if (!dateStr || dateStr.length !== 10) return dateStr || '';
        const y = dateStr.substring(0, 4);
        const m = parseInt(dateStr.substring(5, 7), 10) - 1;
        const d = parseInt(dateStr.substring(8, 10), 10);
        return MONTHS[m] + ' ' + d + ', ' + y;
    }

    async function loadLogs(dateStr) {
        const datesResp = await apiFetch('logs/dates');
        if (datesResp && datesResp.dates) {
            logDates = (datesResp.dates || []).slice().sort();
            if (!logDateNav) {
                logDateNav = new DateNav(
                    'log-date-nav',
                    function (date) {
                        logPagination = {};
                        loadLogContent(date);
                    });

                // Wire up older/newer buttons
                const prevBtn = document.getElementById('log-prev-day-btn');
                const nextBtn = document.getElementById('log-next-day-btn');
                if (prevBtn) {
                    prevBtn.addEventListener('click', () => {
                        const idx = logDates.indexOf(currentLogDate);
                        if (idx > 0) {
                            logPagination = {};
                            loadLogContent(logDates[idx - 1]);
                        }
                    });
                }
                if (nextBtn) {
                    nextBtn.addEventListener('click', () => {
                        const idx = logDates.indexOf(currentLogDate);
                        if (idx >= 0 && idx < logDates.length - 1) {
                            logPagination = {};
                            loadLogContent(logDates[idx + 1]);
                        }
                    });
                }
            }
            logDateNav.setDates(datesResp.dates);
        }
        logPagination = {};
        loadLogContent(dateStr || null);
    }

    async function loadLogContent(dateStr, appendMode, fileLabel, appendOffset) {
        const logEl = document.getElementById('log-content');

        if (!appendMode) {
            logEl.innerHTML = '<div class="log-loading">Loading logs\u2026</div>';
        }

        let endpoint = dateStr
            ? 'logs?date=' + encodeURIComponent(dateStr) + '&lines=' + LOG_PAGE_SIZE
            : 'logs?lines=' + LOG_PAGE_SIZE;

        if (appendMode && appendOffset != null) {
            endpoint += '&offset=' + appendOffset;
        }

        const data = await apiFetch(endpoint);

        if (!appendMode) {
            logEl.innerHTML = '';
        } else {
            // Remove any existing load-earlier button for this label
            const old = logEl.querySelector('.log-load-earlier[data-label="' + (fileLabel || '') + '"]');
            if (old) old.remove();
        }

        const actualDate = (data && data.length > 0) ? (data[0].date || dateStr) : dateStr;

        if (!data || data.length === 0) {
            logEl.innerHTML = '<div class="log-empty">No logs available' + (dateStr ? ' for ' + escHtml(dateStr) : '') + '.</div>';
            
            const dateDisplayEl = document.getElementById('log-current-date-display');
            if (dateDisplayEl) dateDisplayEl.textContent = dateStr || 'No Logs';
            const prevBtn = document.getElementById('log-prev-day-btn');
            const nextBtn = document.getElementById('log-next-day-btn');
            if (prevBtn) prevBtn.disabled = true;
            if (nextBtn) nextBtn.disabled = true;
            return;
        }

        // On fresh load, resolve the actual date loaded
        if (!appendMode) {
            currentLogDate = actualDate;

            // Highlight in DateNav
            if (logDateNav && actualDate) {
                logDateNav.selectDate(actualDate);
            }

            // Update current date display
            const dateDisplayEl = document.getElementById('log-current-date-display');
            if (dateDisplayEl && actualDate) {
                dateDisplayEl.textContent = formatDateHuman(actualDate);
            }

            // Update older/newer button states
            const prevBtn = document.getElementById('log-prev-day-btn');
            const nextBtn = document.getElementById('log-next-day-btn');
            if (actualDate) {
                const idx = logDates.indexOf(actualDate);
                if (prevBtn) prevBtn.disabled = (idx <= 0);
                if (nextBtn) nextBtn.disabled = (idx < 0 || idx >= logDates.length - 1);
            } else {
                if (prevBtn) prevBtn.disabled = true;
                if (nextBtn) nextBtn.disabled = true;
            }
        }

        data.forEach(function(l) {
            const label = l.label || l.file || 'Log';
            const totalLines = l.total_lines || 0;
            const hasMore = l.has_more || false;

            // Track pagination state per file
            if (!appendMode || !logPagination[label]) {
                logPagination[label] = { offset: 0, totalLines: totalLines, date: actualDate };
            }

            // Build file block
            const block = document.createElement('div');
            block.className = 'log-file-block';
            block.dataset.label = label;

            // Header with stats
            const header = document.createElement('div');
            header.className = 'log-file-header';
            const shownLines = (l.content || '').split('\n').length;
            const shownFrom = hasMore ? (totalLines - logPagination[label].offset - shownLines + 1) : 1;
            const shownTo = totalLines - logPagination[label].offset;
            header.innerHTML =
                '<span class="log-file-label">' + escHtml(label) + '</span>' +
                '<span class="log-file-meta">Lines ' + shownFrom + '\u2013' + shownTo +
                ' of ' + totalLines + '</span>' +
                '<button class="log-tail-btn" title="Jump to end" data-label="' + escHtml(label) + '">\u25bc Tail</button>';
            block.appendChild(header);

            // Load Earlier button (only if there are older lines)
            if (hasMore) {
                const earlier = document.createElement('button');
                earlier.className = 'log-load-earlier';
                earlier.dataset.label = label;
                earlier.dataset.date = actualDate || '';
                earlier.textContent = '\u2191 Load earlier lines (' + (totalLines - shownLines - logPagination[label].offset) + ' more)';
                earlier.addEventListener('click', function() {
                    const newOffset = logPagination[label].offset + shownLines;
                    logPagination[label].offset = newOffset;
                    // Load older page and prepend
                    loadLogEarlier(label, actualDate, newOffset, block);
                });
                block.appendChild(earlier);
            }

            // Log content as plain monospace pre — NO markdown parsing
            const pre = document.createElement('pre');
            pre.className = 'log-pre';
            pre.dataset.label = label;
            pre.textContent = l.content || '';
            block.appendChild(pre);

            if (appendMode && fileLabel === label) {
                // Prepend new lines before existing pre
                const existingBlock = logEl.querySelector('.log-file-block[data-label="' + label + '"]');
                if (existingBlock) {
                    existingBlock.replaceWith(block);
                    return;
                }
            }
            logEl.appendChild(block);

            // Auto-scroll to bottom of latest log on first load
            if (!appendMode) {
                pre.scrollTop = pre.scrollHeight;
            }
        });

    }

    async function loadLogEarlier(label, dateStr, offset, existingBlock) {
        const actualDate = dateStr || (logPagination[label] ? logPagination[label].date : null);
        let endpoint = 'logs?lines=' + LOG_PAGE_SIZE + '&offset=' + offset;
        if (actualDate) {
            endpoint += '&date=' + encodeURIComponent(actualDate);
        }

        const data = await apiFetch(endpoint);
        if (!data || !data.length) return;

        const entry = data.find(function(l) { return (l.label || l.file) === label; });
        if (!entry) return;

        const container = document.getElementById('log-content');

        // Remove old earlier button
        const oldBtn = existingBlock.querySelector('.log-load-earlier');
        if (oldBtn) oldBtn.remove();

        const totalLines = entry.total_lines || 0;
        const shownLines = (entry.content || '').split('\n').length;
        const hasMore = entry.has_more || false;

        // Prepend new content to the existing pre
        const existingPre = existingBlock.querySelector('.log-pre');
        if (existingPre) {
            existingPre.textContent = (entry.content || '') + '\n' + existingPre.textContent;
        }

        // Update header stats
        const headerMeta = existingBlock.querySelector('.log-file-meta');
        if (headerMeta) {
            const shownFrom = hasMore ? (totalLines - offset - shownLines + 1) : 1;
            const shownTo = totalLines;
            headerMeta.textContent = 'Lines ' + shownFrom + '\u2013' + shownTo + ' of ' + totalLines;
        }

        // Add new Load Earlier button if still more
        if (hasMore) {
            const earlier = document.createElement('button');
            earlier.className = 'log-load-earlier';
            earlier.dataset.label = label;
            earlier.dataset.date = actualDate || '';
            const remaining = totalLines - offset - shownLines;
            earlier.textContent = '\u2191 Load earlier lines (' + remaining + ' more)';
            earlier.addEventListener('click', function() {
                const newOffset = offset + shownLines;
                logPagination[label].offset = newOffset;
                loadLogEarlier(label, actualDate, newOffset, existingBlock);
            });
            // Insert before the pre element
            if (existingPre) {
                existingBlock.insertBefore(earlier, existingPre);
            }
        }
    }

    // --- Chat ---
    const chatInput =
        document.getElementById('chat-input');
    const chatSend =
        document.getElementById('chat-send');
    const chatMessages =
        document.getElementById('chat-messages');
    const chatSessionList =
        document.getElementById('chat-session-list');
    const chatSessionMeta =
        document.getElementById('chat-session-meta');
    const chatNewSessionBtn =
        document.getElementById('chat-new-session');
    const chatSelectAllBtn =
        document.getElementById('chat-select-all');
    const chatDeleteSelectedBtn =
        document.getElementById(
            'chat-delete-selected');
    const chatSelectionMeta =
        document.getElementById(
            'chat-selection-meta');
    let currentChatSessionId = sessionStorage.getItem('current_chat_session_id') || null;
    let chatSessionsCache = [];
    let selectedChatSessionIds = new Set();

    function formatChatSessionMeta() {
        if (!chatSessionMeta) return;
        if (!currentChatSessionId) {
            chatSessionMeta.textContent =
                'Starting a new chat. Send the first message to create a session.';
            return;
        }
        chatSessionMeta.textContent =
            'Session ' + currentChatSessionId +
            ' is active; continuing conversation.';
    }

    function resetChatMessages() {
        if (!chatMessages) return;
        chatMessages.innerHTML =
            '<div class="chat-welcome">' +
            'Type a message to start chatting ' +
            'with Voxi.</div>';
    }

    function updateChatSelectionMeta() {
        if (!chatSelectionMeta) return;
        const count = selectedChatSessionIds.size;
        if (count === 0) {
            chatSelectionMeta.textContent =
                'No session selected';
            return;
        }
        chatSelectionMeta.textContent =
            count + ' sessions selected';
    }

    function selectChatSession(sessionId) {
        currentChatSessionId = sessionId || null;
        if (currentChatSessionId) {
            sessionStorage.setItem('current_chat_session_id', currentChatSessionId);
        } else {
            sessionStorage.removeItem('current_chat_session_id');
        }
        formatChatSessionMeta();
        if (!chatSessionList) return;
        chatSessionList.querySelectorAll(
            '.chat-session-item').forEach(item => {
                item.classList.toggle('active',
                    item.dataset.sessionId ===
                    currentChatSessionId);
            });
    }

    function renderChatSessionList() {
        if (!chatSessionList) return;
        if (!chatSessionsCache.length) {
            chatSessionList.innerHTML =
                '<p class="empty-state">' +
                'No previous chats yet.</p>';
            selectedChatSessionIds.clear();
            updateChatSelectionMeta();
            return;
        }

        chatSessionList.innerHTML = chatSessionsCache
            .map(session => {
                const isActive =
                    session.id === currentChatSessionId
                        ? ' active' : '';
                const isChecked =
                    selectedChatSessionIds.has(
                        session.id)
                        ? ' checked' : '';
                const preview = escHtml(
                    session.content_preview ||
                    'No preview available.');
                const modified = session.modified
                    ? new Date(session.modified * 1000)
                        .toLocaleString()
                    : '—';
                return '<div class="chat-session-item' +
                    isActive + '" data-session-id="' +
                    escHtml(session.id) + '">' +
                    '<label class="chat-session-check">' +
                    '<input type="checkbox" ' +
                    'data-chat-select="' +
                    escHtml(session.id) + '"' +
                    isChecked + '>' +
                    '</label>' +
                    '<div class="chat-session-body">' +
                    '<div class="chat-session-title">' +
                    escHtml(session.title ||
                        session.id) + '</div>' +
                    '<div class="chat-session-preview">' +
                    preview + '</div>' +
                    '<div class="chat-session-meta">' +
                    escHtml(session.id) + ' · ' +
                    modified + ' · ' +
                    (session.message_count || 0) +
                    ' msgs</div></div>' +
                    '<button class="chat-session-delete" ' +
                    'data-chat-delete="' +
                    escHtml(session.id) + '">' +
                    'Delete</button></div>';
            }).join('');

        chatSessionList.querySelectorAll(
            '.chat-session-item').forEach(item => {
                item.addEventListener('click',
                    async (event) => {
                        if (event.target.closest(
                            '[data-chat-delete]') ||
                            event.target.closest(
                                '[data-chat-select]')) {
                            return;
                        }
                        await loadChatSessionDetail(
                            item.dataset.sessionId);
                    });
            });
        chatSessionList.querySelectorAll(
            '[data-chat-select]').forEach(box => {
                box.addEventListener('change',
                    (event) => {
                        const id =
                            event.target.dataset
                                .chatSelect;
                        if (event.target.checked) {
                            selectedChatSessionIds
                                .add(id);
                        } else {
                            selectedChatSessionIds
                                .delete(id);
                        }
                        updateChatSelectionMeta();
                    });
            });
        chatSessionList.querySelectorAll(
            '[data-chat-delete]').forEach(btn => {
                btn.addEventListener('click',
                    async (event) => {
                        event.stopPropagation();
                        await deleteChatSessions([
                            btn.dataset
                                .chatDelete
                        ]);
                    });
            });
        updateChatSelectionMeta();
    }

    async function loadChatSessions() {
        if (!chatSessionList) return;
        chatSessionList.innerHTML =
            '<p class="empty-state">Loading...</p>';
        const sessions = await apiFetch('sessions');
        if (!Array.isArray(sessions)) {
            chatSessionsCache = [];
            chatSessionList.innerHTML =
                '<p class="empty-state">' +
                'Failed to load previous chats.</p>';
            formatChatSessionMeta();
            return;
        }
        chatSessionsCache = sessions;
        selectedChatSessionIds.forEach(id => {
            if (!chatSessionsCache.some(
                session => session.id === id)) {
                selectedChatSessionIds
                    .delete(id);
            }
        });
        renderChatSessionList();
        formatChatSessionMeta();
    }

    async function deleteChatSessions(ids) {
        const filteredIds = (ids || [])
            .filter(Boolean);
        if (!filteredIds.length) return;
        if (!window.confirm(
            'Delete selected session history?')) {
            return;
        }

        let resp = null;
        if (filteredIds.length === 1) {
            resp = await apiDelete(
                'sessions/' +
                encodeURIComponent(filteredIds[0]));
        } else {
            resp = await apiDelete(
                'sessions', {
                    ids: filteredIds
                });
        }

        if (!resp || !resp.deleted_ids) {
            window.alert('Failed to delete sessions.');
            return;
        }

        resp.deleted_ids.forEach(id => {
            selectedChatSessionIds.delete(id);
            if (currentChatSessionId === id) {
                currentChatSessionId = null;
                resetChatMessages();
            }
        });
        await loadChatSessions();
        selectChatSession(currentChatSessionId);
    }

    async function loadChatSessionDetail(sessionId) {
        if (!chatMessages) return;
        if (window.chatPollInterval) {
            clearInterval(window.chatPollInterval);
            window.chatPollInterval = null;
        }
        const resp = await apiFetch('sessions/' +
            encodeURIComponent(sessionId));
        if (!resp || !Array.isArray(resp.messages)) {
            addChatMsg('assistant',
                'Failed to load session history.');
            return;
        }

        chatMessages.innerHTML = '';
        resp.messages.forEach(message => {
            addChatMsg(message.role, message.text);
        });
        selectChatSession(sessionId);

        // Check if last message is from user, indicating the agent is still running/thinking
        const lastMsg = resp.messages[resp.messages.length - 1];
        if (lastMsg && lastMsg.role === 'user') {
            const activeResp = await apiFetch('chat/active');
            let isActive = false;
            let reqId = 'unknown';
            if (activeResp && Array.isArray(activeResp.active_requests)) {
                const activeReq = activeResp.active_requests.find(r => r.session_id === sessionId);
                if (activeReq) {
                    isActive = true;
                    reqId = activeReq.request_id;
                    sessionStorage.setItem('active_request_id_' + sessionId, reqId);
                }
            }

            if (isActive) {
                showThinkingIndicator(sessionId, reqId);
                startPollingSession(sessionId);
            } else {
                sessionStorage.removeItem('active_request_id_' + sessionId);
            }
        }
    }

    function addChatMsg(role, text) {
        if (!chatMessages) return;
        if (role === 'assistant') {
            text = checkAndShowHookApproval(text);
        }
        const welcome =
            chatMessages.querySelector('.chat-welcome');
        if (welcome) welcome.remove();

        const el = document.createElement('div');
        el.className = 'chat-msg ' + role + (role === 'assistant' ? ' markdown-body' : '');
        if (role === 'assistant' && text.startsWith('⚠️ **Safety Confirmation Required**')) {
            el.classList.add('safety-card');
            el.innerHTML = renderSafetyConfirmation(text);
        } else if (role === 'assistant' && typeof marked !== 'undefined' && typeof marked.parse === 'function') {
            el.innerHTML = marked.parse(text);
        } else {
            el.textContent = text;
        }
        if (role === 'assistant') {
            if (window.__voiceSpeakNext) {
                window.__voiceSpeakNext = false;
                if (typeof speakText === 'function') speakText(text);
            }
            const actions = inferChatActions(text);
            if (actions.length) {
                const actionRow = document.createElement('div');
                actionRow.className = 'chat-action-row';
                actionRow.innerHTML = actions.map(action =>
                    '<button class="chat-action-btn" data-chat-action="' +
                    escHtml(action.prompt) + '">' + escHtml(action.label) +
                    '</button>'
                ).join('');
                el.appendChild(actionRow);
            }
            bindChatActionButtons(el);
        }
        chatMessages.appendChild(el);
        chatMessages.scrollTop =
            chatMessages.scrollHeight;
    }

    function renderSafetyConfirmation(text) {
        const toolMatch = text.match(/\*\*Tool\*\*:\s*`([^`]+)`/);
        const tool = toolMatch ? toolMatch[1] : 'tool action';
        const isOrder = /create|checkout|order|pay|reserve|book/i.test(tool);
        const title = isOrder ? 'Confirm order action' : 'Confirm action';
        const description = isOrder
            ? 'This can place or prepare an order. Voxi needs your final permission before continuing.'
            : 'This step needs your permission before Voxi continues.';

        return '' +
            '<div class="safety-card-header">' +
            '<span class="safety-card-icon">!</span>' +
            '<div><h3>' + title + '</h3><p>' + description + '</p></div>' +
            '</div>' +
            '<div class="safety-actions">' +
            '<button class="btn-primary" data-confirm-reply="Confirm">Confirm</button>' +
            '<button class="btn-outline" data-confirm-reply="Cancel">Cancel</button>' +
            '</div>';
    }

    function inferChatActions(text) {
        const lower = (text || '').toLowerCase();
        const actions = [];
        const add = (label, prompt) => {
            if (!actions.some(action => action.prompt === prompt)) {
                actions.push({ label, prompt });
            }
        };

        if (lower.includes('zepto') && lower.includes('swiggy')) {
            add('Use both', 'both');
        }
        if (lower.includes('cheapest') || lower.includes('options below')) {
            add('Add cheapest', 'choose the cheapest');
        }
        if (lower.includes('qty') || lower.includes('quantity') || lower.includes('added')) {
            add('Set Qty 1', 'set quantity to 1');
        }
        if (lower.includes('cart') || lower.includes('added') || lower.includes('checkout')) {
            add('View cart', 'show cart');
        }
        if (lower.includes('payment') || lower.includes('checkout') || lower.includes('order')) {
            add('Show payment options', 'show payment options');
        }
        if (lower.includes('order') || lower.includes('checkout') || lower.includes('session')) {
            add('Cancel order', 'cancel order');
        }

        return actions.slice(0, 5);
    }

    function bindChatActionButtons(root) {
        root.querySelectorAll('[data-confirm-reply], [data-chat-action]').forEach(btn => {
            btn.addEventListener('click', () => {
                if (!chatInput) return;
                chatInput.value = btn.getAttribute('data-confirm-reply') ||
                    btn.getAttribute('data-chat-action') || '';
                sendChat();
            });
        });
    }

    function showThinkingIndicator(sessionId, requestId) {
        const thinkingId = 'think-' + requestId;
        if (document.getElementById(thinkingId)) return;

        const thinking = document.createElement('div');
        thinking.className = 'chat-thinking';
        thinking.id = thinkingId;
        thinking.innerHTML =
            '<div class="chat-thinking-header">' +
            '<span class="chat-thinking-dot"></span>' +
            '<span class="chat-thinking-dot"></span>' +
            '<span class="chat-thinking-dot"></span>' +
            '<button class="chat-stop-btn" title="Stop generating">Stop</button>' +
            '</div>' +
            '<div class="chat-thinking-logs"></div>';
        chatMessages.appendChild(thinking);
        chatMessages.scrollTop = chatMessages.scrollHeight;

        const stopBtn = thinking.querySelector('.chat-stop-btn');
        if (stopBtn) {
            stopBtn.addEventListener('click', async (e) => {
                e.stopPropagation();
                stopBtn.disabled = true;
                stopBtn.textContent = 'Stopping...';
                try {
                    await apiPost('chat/stop', {
                        session_id: sessionId || currentChatSessionId || '',
                        request_id: requestId
                    });
                    sessionStorage.removeItem('active_request_id_' + sessionId);
                    thinking.remove();
                    if (window.chatPollInterval) {
                        clearInterval(window.chatPollInterval);
                        window.chatPollInterval = null;
                    }
                    await loadChatSessionDetail(sessionId);
                } catch (err) {
                    console.error('Failed to cancel request:', err);
                }
            });
        }
    }

    function startPollingSession(sessionId) {
        if (window.chatPollInterval) {
            clearInterval(window.chatPollInterval);
        }
        let lastOutboundCursor = 0;
        window.chatPollInterval = setInterval(async () => {
            if (sessionId !== currentChatSessionId) {
                clearInterval(window.chatPollInterval);
                window.chatPollInterval = null;
                return;
            }

            // Poll outbound messages to show live logs
            try {
                const outboundResp = await apiFetch('outbound/messages?session_id=' + encodeURIComponent(sessionId) + '&since=' + lastOutboundCursor);
                if (outboundResp && Array.isArray(outboundResp.messages) && outboundResp.messages.length > 0) {
                    const reqId = sessionStorage.getItem('active_request_id_' + sessionId);
                    const thinkingId = 'think-' + (reqId || 'unknown');
                    const thinkingEl = document.getElementById(thinkingId);
                    if (thinkingEl) {
                        let logsEl = thinkingEl.querySelector('.chat-thinking-logs');
                        if (logsEl) {
                            outboundResp.messages.forEach(msg => {
                                const text = msg.message;
                                const exists = Array.from(logsEl.children).some(child => child.textContent === text);
                                if (!exists) {
                                    const logMsgEl = document.createElement('div');
                                    logMsgEl.className = 'chat-thinking-log-entry';
                                    logMsgEl.textContent = text;
                                    logsEl.appendChild(logMsgEl);
                                }
                            });
                            chatMessages.scrollTop = chatMessages.scrollHeight;
                        }
                    }
                    if (typeof outboundResp.cursor === 'number' && outboundResp.cursor >= lastOutboundCursor) {
                        lastOutboundCursor = outboundResp.cursor;
                    }
                }
            } catch (err) {
                console.error('Error fetching thinking logs:', err);
            }

            const resp = await apiFetch('sessions/' + encodeURIComponent(sessionId));
            if (resp && Array.isArray(resp.messages) && resp.messages.length > 0) {
                const lastMsg = resp.messages[resp.messages.length - 1];
                if (lastMsg && lastMsg.role === 'assistant') {
                    clearInterval(window.chatPollInterval);
                    window.chatPollInterval = null;
                    await loadChatSessionDetail(sessionId);
                    await loadChatSessions();
                }
            }
        }, 1500);
    }

    async function sendChat() {
        if (!chatInput || !chatMessages) return;
        const prompt = chatInput.value.trim();
        if (!prompt) return;
        const sessionId = currentChatSessionId;
        const requestId = 'req-' + Date.now() + '-' + Math.random().toString(36).substr(2, 9);

        clearTimeline();
        addChatMsg('user', prompt);
        chatInput.value = '';

        sessionStorage.setItem('active_request_id_' + (sessionId || 'new'), requestId);

        showThinkingIndicator(sessionId || 'new', requestId);

        try {
            const resp = await apiPost('chat', {
                prompt: prompt,
                session_id: sessionId,
                request_id: requestId
            });

            const thinkingId = 'think-' + requestId;
            const indicator = document.getElementById(thinkingId);
            if (indicator) indicator.remove();

            if (resp && resp.session_id) {
                sessionStorage.removeItem('active_request_id_' + sessionId);
                sessionStorage.removeItem('active_request_id_new');
                if (!sessionId) {
                    if (currentChatSessionId === null) {
                        currentChatSessionId = resp.session_id;
                        selectChatSession(resp.session_id);
                    }
                }
            }

            if (resp && resp.response) {
                if (resp.session_id === currentChatSessionId) {
                    addChatMsg('assistant', resp.response);
                }
                await loadChatSessions();
            } else {
                if (resp && resp.session_id === currentChatSessionId) {
                    addChatMsg('assistant',
                        (resp && resp.error) ||
                        'Error: no response from agent.');
                }
            }
        } catch (err) {
            const thinkingId = 'think-' + requestId;
            const indicator = document.getElementById(thinkingId);
            if (indicator) indicator.remove();
            sessionStorage.removeItem('active_request_id_' + sessionId);
            sessionStorage.removeItem('active_request_id_new');
            if (sessionId === currentChatSessionId) {
                addChatMsg('assistant', 'Error: connection failed.');
            }
        }
    }

    if (chatSend) {
        chatSend.addEventListener('click', sendChat);
    }
    if (chatNewSessionBtn) {
        chatNewSessionBtn.addEventListener('click', () => {
            currentChatSessionId = null;
            resetChatMessages();
            selectChatSession(null);
        });
    }
    if (chatSelectAllBtn) {
        chatSelectAllBtn.addEventListener(
            'click', () => {
                if (selectedChatSessionIds.size ===
                    chatSessionsCache.length) {
                    selectedChatSessionIds.clear();
                } else {
                    selectedChatSessionIds =
                        new Set(chatSessionsCache
                            .map(session =>
                                session.id));
                }
                renderChatSessionList();
            });
    }
    if (chatDeleteSelectedBtn) {
        chatDeleteSelectedBtn.addEventListener(
            'click', async () => {
                await deleteChatSessions(
                    Array.from(
                        selectedChatSessionIds));
            });
    }
    if (chatInput) {
        chatInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                sendChat();
            }
        });
    }

    // ==========================
    // Voice input (config-driven)
    // ==========================
    // Engine "browser": STT/TTS run in the browser via Web Speech API (works
    // on the current build). Engine "device": routes to the Rust voxi-voice
    // pipeline (requires the streaming endpoint; falls back with a notice).
    // Selection lives in voice_config.json (editable in Admin > Config) and is
    // overlaid per-browser by localStorage so each client can tweak its voice.
    const chatMic = document.getElementById('chat-mic');
    const voiceConfigBtn = document.getElementById('voice-config-btn');
    const voiceSettings = document.getElementById('voice-settings');
    const voiceEngineSel = document.getElementById('voice-engine');
    const voiceVoiceSel = document.getElementById('voice-voice');
    const voiceLangInput = document.getElementById('voice-lang');
    const voiceSpeakChk = document.getElementById('voice-speak');
    const voiceNote = document.getElementById('voice-settings-note');
    const SpeechRec =
        window.SpeechRecognition || window.webkitSpeechRecognition;
    let voiceRecognition = null;
    let voiceListening = false;
    const VOICE_PREFS_KEY = 'voxi_voice_prefs';

    const voiceConfig = {
        engine: 'browser',
        language: 'en-US',
        speak_replies: true,
        browser_voice: ''
    };

    function loadVoicePrefs() {
        try {
            const raw = localStorage.getItem(VOICE_PREFS_KEY);
            if (raw) Object.assign(voiceConfig, JSON.parse(raw));
        } catch (err) { /* ignore */ }
    }

    function saveVoicePrefs() {
        try {
            localStorage.setItem(VOICE_PREFS_KEY, JSON.stringify({
                engine: voiceConfig.engine,
                language: voiceConfig.language,
                speak_replies: voiceConfig.speak_replies,
                browser_voice: voiceConfig.browser_voice
            }));
        } catch (err) { /* ignore */ }
    }

    async function loadVoiceConfigFile() {
        // Server config is the baseline; local prefs win for this browser.
        try {
            const res = await fetch(API + '/api/voice/config');
            if (res.ok) {
                const data = await res.json();
                const cfg = (data && data.config) || data || {};
                if (cfg.engine) voiceConfig.engine = cfg.engine;
                if (cfg.language) voiceConfig.language = cfg.language;
                if (typeof cfg.speak_replies === 'boolean') {
                    voiceConfig.speak_replies = cfg.speak_replies;
                }
                if (cfg.browser_voice) voiceConfig.browser_voice = cfg.browser_voice;
            }
        } catch (err) { /* endpoint optional; defaults apply */ }
        loadVoicePrefs();
        syncVoiceUI();
    }

    function populateVoiceList() {
        if (!voiceVoiceSel || !('speechSynthesis' in window)) return;
        const voices = window.speechSynthesis.getVoices() || [];
        voiceVoiceSel.innerHTML =
            '<option value="">Default</option>' +
            voices.map(v =>
                '<option value="' + escHtml(v.name) + '">' +
                escHtml(v.name + ' (' + v.lang + ')') + '</option>'
            ).join('');
        voiceVoiceSel.value = voiceConfig.browser_voice || '';
    }

    function syncVoiceUI() {
        if (voiceEngineSel) voiceEngineSel.value = voiceConfig.engine;
        if (voiceLangInput) voiceLangInput.value = voiceConfig.language;
        if (voiceSpeakChk) voiceSpeakChk.checked = !!voiceConfig.speak_replies;
        populateVoiceList();
        if (voiceNote) {
            voiceNote.textContent = voiceConfig.engine === 'device'
                ? 'Device engine uses the Voxi pipeline (requires models + streaming endpoint).'
                : 'Browser engine: STT/TTS run locally in your browser.';
        }
    }

    function pickVoice() {
        if (!('speechSynthesis' in window)) return null;
        const voices = window.speechSynthesis.getVoices() || [];
        if (voiceConfig.browser_voice) {
            return voices.find(v => v.name === voiceConfig.browser_voice) || null;
        }
        return voices.find(v =>
            v.lang === voiceConfig.language) ||
            voices.find(v =>
                v.lang.startsWith((voiceConfig.language || 'en').slice(0, 2))) ||
            null;
    }

    let kokoroModel = null;
    async function loadKokoroLibrary() {
        if (kokoroModel) return kokoroModel;
        const { pipeline } = await import('https://cdn.jsdelivr.net/npm/@huggingface/transformers@3.0.0-alpha.5');
        kokoroModel = await pipeline('text-to-speech', 'onnx-community/Kokoro-82M-v1.0-ONNX', {
            dtype: 'fp32',
            device: 'webgpu'
        });
        return kokoroModel;
    }

    async function speakText(raw) {
        if (!voiceConfig.speak_replies) return;
        if (!raw) return;
        const spoken = raw
            .replace(/```[\s\S]*?```/g, ' code block ')
            .replace(/[#*_`>~|]/g, '')
            .replace(/\[(.*?)\]\(.*?\)/g, '$1')
            .replace(/\s+/g, ' ')
            .trim();
        if (!spoken) return;

        if (voiceConfig.engine === 'kokoro_browser') {
            try {
                const model = await loadKokoroLibrary();
                const output = await model(spoken, {
                    speaker: voiceConfig.browser_voice || 'af_bella'
                });
                const audioContext = new (window.AudioContext || window.webkitAudioContext)();
                const audioBuffer = audioContext.createBuffer(1, output.audio.length, output.sampling_rate);
                audioBuffer.getChannelData(0).set(output.audio);
                const source = audioContext.createBufferSource();
                source.buffer = audioBuffer;
                source.connect(audioContext.destination);
                source.start(0);
                return;
            } catch (err) {
                console.warn('Client-side neural speech (kokoro_browser) failed, falling back to Web Speech API:', err);
            }
        }

        try {
            window.speechSynthesis.cancel();
            const utter = new SpeechSynthesisUtterance(spoken);
            utter.lang = voiceConfig.language || 'en-US';
            const v = pickVoice();
            if (v) utter.voice = v;
            window.speechSynthesis.speak(utter);
        } catch (err) {
            console.error('TTS failed:', err);
        }
    }


    function setMicState(listening) {
        voiceListening = listening;
        if (chatMic) chatMic.classList.toggle('listening', listening);
    }

    function startBrowserMic() {
        if (!SpeechRec) return;
        if (voiceListening && voiceRecognition) {
            voiceRecognition.stop();
            return;
        }
        voiceRecognition = new SpeechRec();
        voiceRecognition.lang = voiceConfig.language || 'en-US';
        voiceRecognition.interimResults = false;
        voiceRecognition.maxAlternatives = 1;
        voiceRecognition.onstart = () => setMicState(true);
        voiceRecognition.onend = () => setMicState(false);
        voiceRecognition.onerror = (e) => {
            setMicState(false);
            if (e.error === 'not-allowed' || e.error === 'service-not-allowed') {
                addChatMsg('assistant',
                    'Microphone permission was denied. Allow mic access in your browser to use voice input.');
            } else if (e.error !== 'aborted' && e.error !== 'no-speech') {
                console.error('Speech recognition error:', e.error);
            }
        };
        voiceRecognition.onresult = (e) => {
            const transcript = (e.results && e.results[0] &&
                e.results[0][0] && e.results[0][0].transcript || '').trim();
            if (!transcript || !chatInput) return;
            chatInput.value = transcript;
            window.__voiceSpeakNext = true;
            sendChat();
        };
        try {
            voiceRecognition.start();
        } catch (err) {
            setMicState(false);
            console.error('Could not start mic:', err);
        }
    }

    if (chatMic) {
        if (!SpeechRec) {
            chatMic.disabled = true;
            chatMic.title =
                'Voice input not supported in this browser (try Chrome/Edge/Safari)';
        } else {
            chatMic.addEventListener('click', () => {
                if (voiceConfig.engine === 'device') {
                    addChatMsg('assistant',
                        'Device voice engine is selected but its streaming endpoint is not available on this build. Switch to the Browser engine in voice settings, or install voice models for the device pipeline.');
                    return;
                }
                startBrowserMic();
            });
        }
    }

    if (voiceConfigBtn && voiceSettings) {
        voiceConfigBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            voiceSettings.classList.toggle('hidden');
            if (!voiceSettings.classList.contains('hidden')) populateVoiceList();
        });
        document.addEventListener('click', (e) => {
            if (!voiceSettings.contains(e.target) && e.target !== voiceConfigBtn) {
                voiceSettings.classList.add('hidden');
            }
        });
    }
    if (voiceEngineSel) {
        voiceEngineSel.addEventListener('change', () => {
            voiceConfig.engine = voiceEngineSel.value;
            saveVoicePrefs();
            syncVoiceUI();
        });
    }
    if (voiceVoiceSel) {
        voiceVoiceSel.addEventListener('change', () => {
            voiceConfig.browser_voice = voiceVoiceSel.value;
            saveVoicePrefs();
        });
    }
    if (voiceLangInput) {
        voiceLangInput.addEventListener('change', () => {
            voiceConfig.language = voiceLangInput.value.trim() || 'en-US';
            saveVoicePrefs();
        });
    }
    if (voiceSpeakChk) {
        voiceSpeakChk.addEventListener('change', () => {
            voiceConfig.speak_replies = voiceSpeakChk.checked;
            saveVoicePrefs();
        });
    }
    if ('speechSynthesis' in window) {
        window.speechSynthesis.onvoiceschanged = populateVoiceList;
    }
    loadVoiceConfigFile();

    // ==========================
    // Slash-command palette
    // ==========================
    // Typing "/" at the start of the chat input shows client commands plus
    // the agent's live tools/workflows/roles (from /api/capabilities).
    const slashPalette = document.getElementById('slash-palette');
    let slashItems = [];
    let slashActiveIdx = 0;
    let capabilitiesCache = null;
    let skillsCache = null;
    let paletteFilter = '';

    const CLIENT_COMMANDS = [
        { cmd: '/help', label: 'Show available commands & tools', kind: 'command' },
        { cmd: '/new', label: 'Start a new chat session', kind: 'command' },
        { cmd: '/clear', label: 'Clear messages on screen', kind: 'command' },
        { cmd: '/voice', label: 'Toggle speaking replies aloud', kind: 'command' },
        { cmd: '/stop', label: 'Stop speaking / listening', kind: 'command' }
    ];

    async function loadCapabilities() {
        if (capabilitiesCache) return capabilitiesCache;
        try {
            const res = await fetch(API + '/api/capabilities');
            if (res.ok) {
                capabilitiesCache = await res.json();
            } else {
                capabilitiesCache = { tools: [], workflows: [], roles: [] };
            }
        } catch (err) {
            capabilitiesCache = { tools: [], workflows: [], roles: [] };
        }
        return capabilitiesCache;
    }

    function buildSlashItems(caps) {
        const items = CLIENT_COMMANDS.map(c => ({
            type: 'command', key: c.cmd, label: c.label, group: 'Commands'
        }));
        (caps.tools || []).forEach(t => {
            const name = t.name || t;
            items.push({
                type: 'tool', key: name,
                label: t.description || 'Tool',
                group: 'Tools'
            });
        });
        (caps.workflows || []).forEach(w => {
            const name = w.name || w;
            items.push({
                type: 'workflow', key: name,
                label: w.description || 'Workflow', group: 'Workflows'
            });
        });
        return items;
    }

    async function loadSkillsList() {
        if (skillsCache) return skillsCache;
        try {
            const res = await fetch(API + '/api/skills');
            const j = res.ok ? await res.json() : {};
            skillsCache = (j.skills || []).map(s => ({
                name: s.name, desc: s.description || ''
            }));
        } catch (e) { skillsCache = []; }
        return skillsCache;
    }

    function buildMentionItems(skills) {
        return (skills || []).map(s => ({
            type: 'skill', key: s.name,
            label: s.desc || 'Skill', group: 'Skills (@mention)'
        }));
    }

    function renderSlash(filter) {
        if (!slashPalette) return;
        const f = (filter || '').toLowerCase();
        const matches = slashItems.filter(it =>
            it.key.toLowerCase().includes(f) ||
            (it.label || '').toLowerCase().includes(f));
        if (!matches.length) {
            slashPalette.classList.add('hidden');
            return;
        }
        slashActiveIdx = Math.min(slashActiveIdx, matches.length - 1);
        let html = '';
        let lastGroup = null;
        matches.forEach((it, i) => {
            if (it.group !== lastGroup) {
                html += '<div class="slash-group">' + escHtml(it.group) + '</div>';
                lastGroup = it.group;
            }
            html += '<div class="slash-item' +
                (i === slashActiveIdx ? ' active' : '') +
                '" data-idx="' + i + '">' +
                '<span class="slash-key">' + escHtml(it.key) + '</span>' +
                '<span class="slash-label">' + escHtml(it.label) + '</span></div>';
        });
        slashPalette.innerHTML = html;
        slashPalette.classList.remove('hidden');
        slashPalette._matches = matches;
        slashPalette.querySelectorAll('.slash-item').forEach(el => {
            el.addEventListener('mousedown', (e) => {
                e.preventDefault();
                chooseSlash(matches[parseInt(el.dataset.idx, 10)]);
            });
        });
    }

    function closeSlash() {
        if (slashPalette) slashPalette.classList.add('hidden');
        slashActiveIdx = 0;
    }

    function chooseSlash(item) {
        if (!item || !chatInput) return;
        closeSlash();
        if (item.type === 'command') {
            runClientCommand(item.key);
            chatInput.value = '';
            return;
        }
        if (item.type === 'skill') {
            // Replace the trailing @token with the chosen skill mention so the
            // agent force-prefetches that skill for this prompt.
            chatInput.value = chatInput.value.replace(/@[\w-]*$/, '@' + item.key + ' ');
            chatInput.focus();
            return;
        }
        // Tool/workflow: insert a starter prompt for the user to complete.
        chatInput.value = 'Use ' + item.key + ' to ';
        chatInput.focus();
    }

    function runClientCommand(cmd) {
        switch (cmd) {
            case '/new':
                currentChatSessionId = null;
                resetChatMessages();
                selectChatSession(null);
                break;
            case '/clear':
                resetChatMessages();
                break;
            case '/voice':
                voiceConfig.speak_replies = !voiceConfig.speak_replies;
                saveVoicePrefs();
                syncVoiceUI();
                addChatMsg('assistant', 'Speaking replies is now ' +
                    (voiceConfig.speak_replies ? 'ON' : 'OFF') + '.');
                break;
            case '/stop':
                if ('speechSynthesis' in window) window.speechSynthesis.cancel();
                if (voiceListening && voiceRecognition) voiceRecognition.stop();
                break;
            case '/help':
            default:
                loadCapabilities().then(caps => {
                    const tools = (caps.tools || []).map(t => '- `' +
                        (t.name || t) + '`').join('\n') || '_none_';
                    addChatMsg('assistant',
                        '**Commands**\n' +
                        CLIENT_COMMANDS.map(c => '- `' + c.cmd + '` — ' +
                            c.label).join('\n') +
                        '\n\n**Tools**\n' + tools);
                });
                break;
        }
    }

    if (chatInput && slashPalette) {
        chatInput.addEventListener('input', async () => {
            const val = chatInput.value;
            // `@word` as the most recent token → skill mention picker.
            const mention = val.match(/(^|\s)@([\w-]*)$/);
            if (val.startsWith('/') && !val.includes('\n')) {
                slashItems = buildSlashItems(await loadCapabilities());
                paletteFilter = val.slice(1);
                renderSlash(paletteFilter);
            } else if (mention) {
                slashItems = buildMentionItems(await loadSkillsList());
                paletteFilter = mention[2];
                renderSlash(paletteFilter);
            } else {
                closeSlash();
            }
        });
        // Capture phase so palette navigation wins over the send-on-Enter handler.
        chatInput.addEventListener('keydown', (e) => {
            if (slashPalette.classList.contains('hidden')) return;
            const matches = slashPalette._matches || [];
            if (e.key === 'ArrowDown') {
                e.preventDefault(); e.stopPropagation();
                slashActiveIdx = (slashActiveIdx + 1) % matches.length;
                renderSlash(paletteFilter);
            } else if (e.key === 'ArrowUp') {
                e.preventDefault(); e.stopPropagation();
                slashActiveIdx = (slashActiveIdx - 1 + matches.length) % matches.length;
                renderSlash(paletteFilter);
            } else if (e.key === 'Enter') {
                e.preventDefault(); e.stopPropagation();
                chooseSlash(matches[slashActiveIdx]);
            } else if (e.key === 'Escape') {
                e.stopPropagation();
                closeSlash();
            }
        }, true);
    }

    // ==========================
    // Admin Page
    // ==========================

    const CONFIG_LABELS = {
        'llm_config.json': 'LLM Configuration',
        'mcp_servers.json': 'MCP Servers',
        'telegram_config.json': 'Telegram Bot',
        'slack_config.json': 'Slack Integration',
        'discord_config.json': 'Discord Bot',
        'webhook_config.json': 'Webhook Routes',
        'tool_policy.json': 'Tool Policy',
        'agent_roles.json': 'Agent Roles',
        'tunnel_config.json': 'Tunnel Configuration',
        'web_search_config.json': 'Web Search',
        'voice_config.json': 'Voice Configuration',
        'hooks.json': 'Tool Hooks',
        'skills_state.json': 'Skills Activation',
        'system_prompt.txt': 'System Prompt',
        'channel_config.json': 'Channel Settings',
        'channels.json': 'Active Channels',
        'autonomous_trigger.json': 'Autonomous Triggers',
        'device_profile.json': 'Device Profile',
        'fleet_config.json': 'Fleet Config',
        'llm_config_lmstudio.json': 'LM Studio Config',
        'llm_config_mlx.json': 'MLX Config',
        'memory_config.json': 'Memory Config',
        'models.voice.json': 'Voice Models',
        'offline_fallback.json': 'Offline Fallback',
        'safety_bounds.json': 'Safety Bounds',
        'user_profiles.json': 'User Profiles'
    };
    const CONFIG_DESCRIPTIONS = {
        'llm_config.json': 'Manage model backends, token limits, and sampling options.',
        'mcp_servers.json': 'Configure MCP server commands, HTTP endpoints, and tool discovery.',
        'telegram_config.json': 'Configure the Telegram bot token and channel bindings.',
        'slack_config.json': 'Configure Slack app tokens, bot tokens, and channel bindings.',
        'discord_config.json': 'Adjust Discord bot credentials and connection settings.',
        'webhook_config.json': 'Control webhook endpoints and routing policies.',
        'tool_policy.json': 'Manage allowed tools and execution policies.',
        'agent_roles.json': 'Define agent roles and prompt routing behavior.',
        'tunnel_config.json': 'Manage tunnel endpoints and authentication tokens.',
        'web_search_config.json': 'Configure search providers and search options.',
        'voice_config.json': 'Select the voice engine, language, and spoken-reply preferences.',
        'hooks.json': 'Pre/post-tool hooks with allow, deny, and ask approval gates.',
        'skills_state.json': 'Enable or disable skills and opt into project-level skill scanning.',
        'system_prompt.txt': 'Edit the core system instructions and behavioral constraints of the agent.',
        'channel_config.json': 'Configure system notification and control interfaces.',
        'channels.json': 'Manage mapping of channels, config files, and verification criteria.',
        'autonomous_trigger.json': 'Set rules and rates for running background tasks automatically.',
        'device_profile.json': 'Adjust device-specific capabilities, features, and specs.',
        'fleet_config.json': 'Configure fleet settings, endpoints, and secure credentials.',
        'llm_config_lmstudio.json': 'Configure LM Studio endpoint options and model selections.',
        'llm_config_mlx.json': 'Tune local Apple Silicon MLX model features and pathing.',
        'memory_config.json': 'Manage memory storage limits, search parameters, and chunking.',
        'models.voice.json': 'List and verify local or remote voice models and features.',
        'offline_fallback.json': 'Define alternate models and behaviors when internet is offline.',
        'safety_bounds.json': 'Limit AI execution depth, confirm irreversible actions, and restrict parameters.',
        'user_profiles.json': 'Manage administrator credentials, API keys, and personalized parameters.'
    };
    let adminConfigsCache = [];
    let activeConfigName = null;
    let activeConfigParsed = null;

    async function loadAdmin() {
        if (!authToken) {
            showLoginForm();
            return;
        }
        if (await ensureAdminSession()) {
            showAdminPanel();
        }
    }

    function showLoginForm(message) {
        document.getElementById('admin-login')
            .style.display = '';
        document.getElementById('admin-panel')
            .style.display = 'none';
        closeConfigModal();
        document.getElementById(
            'login-error').textContent =
            message || '';
    }

    function showAdminPanel() {
        document.getElementById('admin-login')
            .style.display = 'none';
        document.getElementById('admin-panel')
            .style.display = '';
        loadConfigs();
    }

    function handleAdminUnauthorized(message) {
        clearAdminToken();
        showLoginForm(message || 'Session expired');
    }

    async function ensureAdminSession() {
        if (!authToken) return false;
        const resp = await apiFetch('auth/session');
        return !!(resp && resp.status === 'ok');
    }

    // --- Login ---
    document.getElementById('admin-login-btn')
        .addEventListener('click', doLogin);
    document.getElementById('admin-password')
        .addEventListener('keydown', (e) => {
            if (e.key === 'Enter') doLogin();
        });

    async function doLogin() {
        const pw = document.getElementById(
            'admin-password').value;
        const errEl = document.getElementById(
            'login-error');
        errEl.textContent = '';

        if (!pw) {
            errEl.textContent = 'Password required';
            return;
        }

        const resp = await apiPost(
            'auth/login', { password: pw });

        if (resp && resp.status === 'ok') {
            persistAdminToken(resp.token);
            document.getElementById(
                'admin-password').value = '';
            showAdminPanel();
            showToast('Admin session ready',
                'success');
        } else {
            errEl.textContent =
                (resp && resp.error) ||
                'Login failed';
        }
    }

    // --- Logout ---
    document.getElementById('admin-logout-btn')
        .addEventListener('click', async () => {
            await apiPost('auth/logout', {});
            clearAdminToken();
            showLoginForm();
        });

    // --- Password Change ---
    document.getElementById('admin-change-pw-btn')
        .addEventListener('click', () => {
            const f = document.getElementById(
                'pw-change-form');
            f.style.display =
                f.style.display === 'none' ? '' : 'none';
        });

    document.getElementById('pw-cancel-btn')
        .addEventListener('click', () => {
            document.getElementById('pw-change-form')
                .style.display = 'none';
        });

    document.getElementById('pw-save-btn')
        .addEventListener('click', async () => {
            const cur = document.getElementById(
                'pw-current').value;
            const nw = document.getElementById(
                'pw-new').value;
            const msg = document.getElementById(
                'pw-change-msg');

            if (!cur || !nw) {
                msg.textContent = 'Fill in both fields';
                msg.style.color = 'var(--danger)';
                return;
            }

            const resp = await apiPost(
                'auth/change_password', {
                current_password: cur,
                new_password: nw
            });

            if (resp && resp.status === 'ok') {
                msg.textContent = 'Password changed!';
                msg.style.color = 'var(--success)';
                document.getElementById(
                    'pw-current').value = '';
                document.getElementById(
                    'pw-new').value = '';
                setTimeout(() => {
                    document.getElementById(
                        'pw-change-form').style.display =
                        'none';
                    msg.textContent = '';
                }, 2000);
            } else {
                msg.textContent =
                    (resp && resp.error) || 'Failed';
                msg.style.color = 'var(--danger)';
            }
        });

    // --- Config Management ---
    // ── Config layout (prioritized primary grid + "Less Used" accordion) ──
    const CONFIG_LAYOUT_KEY = 'voxi_config_layout';
    const DEFAULT_PRIMARY_CONFIGS = [
        'llm_config.json', 'mcp_servers.json', 'agent_roles.json',
        'tool_policy.json', 'hooks.json', 'voice_config.json',
        'skills_state.json', 'channel_config.json'
    ];
    const DEFAULT_HIDDEN_CONFIGS = [
        'telegram_config.json', 'slack_config.json', 'discord_config.json',
        'webhook_config.json', 'tunnel_config.json', 'web_search_config.json',
        'system_prompt.txt', 'channels.json', 'autonomous_trigger.json',
        'device_profile.json', 'fleet_config.json', 'llm_config_lmstudio.json',
        'llm_config_mlx.json', 'memory_config.json', 'models.voice.json',
        'offline_fallback.json', 'safety_bounds.json', 'user_profiles.json'
    ];

    function loadConfigLayout() {
        try {
            const saved = JSON.parse(localStorage.getItem(CONFIG_LAYOUT_KEY));
            if (saved && Array.isArray(saved.primary) &&
                Array.isArray(saved.hidden)) {
                return { primary: saved.primary, hidden: saved.hidden };
            }
        } catch (e) { /* fall through to defaults */ }
        return {
            primary: DEFAULT_PRIMARY_CONFIGS.slice(),
            hidden: DEFAULT_HIDDEN_CONFIGS.slice()
        };
    }

    function saveConfigLayout(layout) {
        try {
            localStorage.setItem(CONFIG_LAYOUT_KEY, JSON.stringify(layout));
        } catch (e) { /* storage unavailable; non-fatal */ }
    }

    // Keep the saved layout in sync with what the server actually exposes:
    // drop unknown names, append any newly-available configs to primary.
    function reconcileConfigLayout(layout, available) {
        const avail = new Set(available);
        const seen = new Set();
        const primary = layout.primary.filter(
            n => avail.has(n) && !seen.has(n) && seen.add(n));
        const hidden = layout.hidden.filter(
            n => avail.has(n) && !seen.has(n) && seen.add(n));
        available.forEach(n => { if (!seen.has(n)) { primary.push(n); seen.add(n); } });
        return { primary, hidden };
    }

    let configLayout = loadConfigLayout();

    function configCardHtml(c, hidden) {
        const label = CONFIG_LABELS[c.name] || c.name;
        const statusClass = c.exists ? 'exists' : 'missing';
        const statusText = c.exists ? '● Active' : '○ Sample';
        const handle = hidden ? '' :
            '<span class="config-drag-handle" title="Drag to reorder" ' +
            'aria-hidden="true">⠿</span>';
        const toggleLabel = hidden ? 'Show' : 'Hide';
        const toggleIcon = hidden ? '👁' : '🚫';
        return '<div class="config-card" draggable="' +
            (hidden ? 'false' : 'true') + '" data-config="' +
            escHtml(c.name) + '">' +
            handle +
            '<div class="config-card-header">' +
            '<div class="config-card-copy">' +
            '<span class="config-card-title">' + escHtml(label) + '</span>' +
            '<p class="config-card-desc">' +
            escHtml(CONFIG_DESCRIPTIONS[c.name] || 'Configuration editor') +
            '</p></div>' +
            '<div class="config-card-side">' +
            '<span class="config-card-status ' + statusClass + '">' +
            statusText + '</span>' +
            '<button type="button" class="config-card-toggle" ' +
            'data-toggle="' + escHtml(c.name) + '" ' +
            'title="' + toggleLabel + ' this setting">' + toggleIcon +
            '</button>' +
            '<span class="config-card-open">Open</span>' +
            '</div></div></div>';
    }

    async function loadConfigs() {
        const list = document.getElementById('config-list');
        const data = await apiFetch('config/list');

        if (!data || !data.configs) {
            list.innerHTML =
                '<p class="empty-state">Failed to load configs</p>';
            return;
        }

        adminConfigsCache = data.configs.slice();
        const byName = {};
        data.configs.forEach(c => { byName[c.name] = c; });
        configLayout = reconcileConfigLayout(
            configLayout, data.configs.map(c => c.name));
        saveConfigLayout(configLayout);

        const primaryCards = configLayout.primary
            .map(n => configCardHtml(byName[n], false)).join('');
        const hiddenCards = configLayout.hidden
            .map(n => configCardHtml(byName[n], true)).join('') ||
            '<p class="empty-state">No hidden settings.</p>';

        list.innerHTML =
            '<div id="config-primary-list" class="config-primary-list">' +
            primaryCards + '</div>' +
            '<div id="config-less-used-section" class="config-less-used-section">' +
            '<button type="button" id="config-less-used-header" ' +
            'class="config-less-used-header">' +
            '<span class="config-less-used-chevron">▶</span>' +
            '<span>Less Used Settings</span>' +
            '<span class="config-less-used-count">' +
            configLayout.hidden.length + '</span></button>' +
            '<div id="config-less-used-list" class="config-less-used-list">' +
            hiddenCards + '</div></div>';

        wireConfigCardEvents(list);
    }

    function wireConfigCardEvents(list) {
        // Open the editor when the card body (not a control) is clicked.
        list.querySelectorAll('.config-card').forEach(card => {
            card.addEventListener('click', (ev) => {
                if (ev.target.closest('.config-card-toggle')) return;
                openConfigModal(card.dataset.config);
            });
        });

        // Hide/Show: move a config between primary and hidden lists.
        list.querySelectorAll('.config-card-toggle').forEach(btn => {
            btn.addEventListener('click', (ev) => {
                ev.stopPropagation();
                const name = btn.dataset.toggle;
                const pi = configLayout.primary.indexOf(name);
                if (pi >= 0) {
                    configLayout.primary.splice(pi, 1);
                    configLayout.hidden.push(name);
                } else {
                    const hi = configLayout.hidden.indexOf(name);
                    if (hi >= 0) configLayout.hidden.splice(hi, 1);
                    configLayout.primary.push(name);
                }
                saveConfigLayout(configLayout);
                loadConfigs();
            });
        });

        // Collapsible accordion toggle.
        const header = list.querySelector('#config-less-used-header');
        const section = list.querySelector('#config-less-used-section');
        if (header && section) {
            header.addEventListener('click', () => {
                section.classList.toggle('open');
            });
        }

        // HTML5 drag-and-drop reordering within the primary grid.
        const primaryList = list.querySelector('#config-primary-list');
        if (!primaryList) return;
        let dragName = null;
        primaryList.querySelectorAll('.config-card').forEach(card => {
            card.addEventListener('dragstart', (ev) => {
                dragName = card.dataset.config;
                card.classList.add('dragging');
                ev.dataTransfer.effectAllowed = 'move';
            });
            card.addEventListener('dragend', () => {
                card.classList.remove('dragging');
                primaryList.querySelectorAll('.config-card-dropzone')
                    .forEach(c => c.classList.remove('config-card-dropzone'));
            });
            card.addEventListener('dragover', (ev) => {
                ev.preventDefault();
                if (card.dataset.config !== dragName) {
                    card.classList.add('config-card-dropzone');
                }
            });
            card.addEventListener('dragleave', () => {
                card.classList.remove('config-card-dropzone');
            });
            card.addEventListener('drop', (ev) => {
                ev.preventDefault();
                card.classList.remove('config-card-dropzone');
                const target = card.dataset.config;
                if (!dragName || dragName === target) return;
                const arr = configLayout.primary;
                const from = arr.indexOf(dragName);
                const to = arr.indexOf(target);
                if (from < 0 || to < 0) return;
                arr.splice(from, 1);
                arr.splice(to, 0, dragName);
                saveConfigLayout(configLayout);
                loadConfigs();
            });
        });
    }

    async function fetchConfigContent(name) {
        const resp = await apiFetch(
            'config/' + name);

        if (resp && resp.status === 'ok') {
            return {
                ok: true,
                exists: true,
                content: resp.content
            };
        }
        if (resp && resp.sample) {
            return {
                ok: true,
                exists: false,
                content: resp.sample,
                message: 'No config found — sample loaded'
            };
        }
        return {
            ok: false,
            error: (resp && resp.error) ||
                'Load failed'
        };
    }

    function tryParseJson(content) {
        try {
            return JSON.parse(content);
        } catch (e) {
            return null;
        }
    }

    function renderConfigField(key, value) {
        const type = Array.isArray(value)
            ? 'array'
            : value === null
                ? 'null'
                : typeof value;

        if (type === 'boolean') {
            return '<label class="config-field">' +
                '<span class="config-field-label">' +
                escHtml(key) + '</span>' +
                '<select class="config-field-input"' +
                ' data-config-key="' + escHtml(key) + '"' +
                ' data-config-type="boolean">' +
                '<option value="true"' +
                (value ? ' selected' : '') +
                '>true</option>' +
                '<option value="false"' +
                (!value ? ' selected' : '') +
                '>false</option></select></label>';
        }

        if (type === 'number') {
            return '<label class="config-field">' +
                '<span class="config-field-label">' +
                escHtml(key) + '</span>' +
                '<input type="number" class="config-field-input"' +
                ' data-config-key="' + escHtml(key) + '"' +
                ' data-config-type="number" value="' +
                escHtml(String(value)) + '"></label>';
        }

        if (type === 'object' || type === 'array') {
            return '<label class="config-field">' +
                '<span class="config-field-label">' +
                escHtml(key) + '</span>' +
                '<textarea class="config-field-input config-field-code"' +
                ' data-config-key="' + escHtml(key) + '"' +
                ' data-config-type="json">' +
                escHtml(JSON.stringify(value, null, 2)) +
                '</textarea></label>';
        }

        return '<label class="config-field">' +
            '<span class="config-field-label">' +
            escHtml(key) + '</span>' +
            '<textarea class="config-field-input"' +
            ' data-config-key="' + escHtml(key) + '"' +
            ' data-config-type="string">' +
            escHtml(value === null ? '' : String(value)) +
            '</textarea></label>';
    }

    function renderConfigStructuredEditor() {
        const fields = document.getElementById(
            'config-modal-fields');
        const helper = document.getElementById(
            'config-modal-helper');

        if (!activeConfigParsed ||
            typeof activeConfigParsed !== 'object' ||
            Array.isArray(activeConfigParsed)) {
            fields.innerHTML =
                '<p class="empty-state">Structured editing is available only for JSON objects.</p>';
            helper.textContent =
                'Use the raw editor to update the full document.';
            return;
        }

        const entries =
            Object.entries(activeConfigParsed);
        helper.textContent =
            'Update top-level fields here, then save the configuration.';
        fields.innerHTML = entries.length
            ? entries.map(([key, value]) =>
                renderConfigField(key, value)).join('')
            : '<p class="empty-state">No editable fields were found.</p>';
    }

    function setConfigModalMode(mode) {
        const structured = document.getElementById(
            'config-modal-structured');
        const raw = document.getElementById(
            'config-modal-raw-wrap');
        const structuredTab = document.getElementById(
            'config-tab-structured');
        const rawTab = document.getElementById(
            'config-tab-raw');
        const canUseStructured = !!(
            activeConfigParsed &&
            typeof activeConfigParsed === 'object' &&
            !Array.isArray(activeConfigParsed));

        if (mode === 'structured' &&
            !canUseStructured) {
            mode = 'raw';
        }

        if (mode === 'raw' &&
            structuredTab.classList.contains(
                'active')) {
            try {
                document.getElementById(
                    'config-modal-raw').value =
                    collectStructuredConfig();
            } catch (e) {
                document.getElementById(
                    'config-modal-msg').textContent =
                    'Unable to switch to raw view: ' +
                    e.message;
                document.getElementById(
                    'config-modal-msg').className =
                    'config-modal-msg error';
                return;
            }
        }

        if (mode === 'structured' &&
            rawTab.classList.contains('active')) {
            const parsed = tryParseJson(
                document.getElementById(
                    'config-modal-raw').value);
            if (!parsed ||
                typeof parsed !== 'object' ||
                Array.isArray(parsed)) {
                document.getElementById(
                    'config-modal-msg').textContent =
                    'Structured view requires a JSON object.';
                document.getElementById(
                    'config-modal-msg').className =
                    'config-modal-msg error';
                return;
            }
            activeConfigParsed = parsed;
            renderConfigStructuredEditor();
        }

        structured.style.display =
            mode === 'structured' ? '' : 'none';
        raw.style.display =
            mode === 'raw' ? '' : 'none';
        structuredTab.classList.toggle(
            'active', mode === 'structured');
        rawTab.classList.toggle(
            'active', mode === 'raw');
        structuredTab.disabled =
            !canUseStructured;
    }

    async function openConfigModal(name) {
        const modal = document.getElementById(
            'config-modal');
        const msg = document.getElementById(
            'config-modal-msg');
        const title = document.getElementById(
            'config-modal-title');
        const file = document.getElementById(
            'config-modal-name');
        const status = document.getElementById(
            'config-modal-status');
        const format = document.getElementById(
            'config-modal-format');
        const raw = document.getElementById(
            'config-modal-raw');

        activeConfigName = name;
        title.textContent =
            CONFIG_LABELS[name] || name;
        file.textContent = name;
        msg.textContent = 'Loading...';
        msg.className = 'config-modal-msg';
        modal.classList.add('open');
        document.body.classList.add('modal-open');

        const loaded = await fetchConfigContent(name);
        if (!loaded.ok) {
            msg.textContent = loaded.error;
            msg.className =
                'config-modal-msg error';
            return;
        }

        raw.value = loaded.content || '';
        activeConfigParsed = tryParseJson(raw.value);
        status.textContent = loaded.exists
            ? 'Active'
            : 'Sample';
        status.className = 'config-chip ' +
            (loaded.exists ? 'success' :
                'warning');
        format.textContent = activeConfigParsed
            ? 'JSON'
            : 'TEXT';
        renderConfigStructuredEditor();
        setConfigModalMode('structured');

        if (loaded.message) {
            msg.textContent = loaded.message;
            msg.className =
                'config-modal-msg warning';
        } else {
            msg.textContent = '';
            msg.className =
                'config-modal-msg';
        }
    }

    function closeConfigModal() {
        const modal = document.getElementById(
            'config-modal');
        if (modal) {
            modal.classList.remove('open');
        }
        document.body.classList.remove('modal-open');
        activeConfigName = null;
        activeConfigParsed = null;
    }

    function collectStructuredConfig() {
        const next = {};
        const inputs = document.querySelectorAll(
            '#config-modal-fields [data-config-key]');

        for (const input of inputs) {
            const key = input.dataset.configKey;
            const type = input.dataset.configType;
            let value = input.value;

            if (type === 'boolean') {
                value = value === 'true';
            } else if (type === 'number') {
                if (value.trim() === '' ||
                    Number.isNaN(Number(value))) {
                    throw new Error(
                        key + ' must be numeric');
                }
                value = Number(value);
            } else if (type === 'json') {
                value = JSON.parse(value);
            }

            next[key] = value;
        }

        return JSON.stringify(next, null, 2);
    }

    async function saveConfig(name) {
        const msg = document.getElementById(
            'config-modal-msg');
        const structuredTab = document.getElementById(
            'config-tab-structured');
        const rawEditor = document.getElementById(
            'config-modal-raw');
        let content = rawEditor.value;

        try {
            if (structuredTab.classList.contains(
                'active')) {
                content = collectStructuredConfig();
            } else if (tryParseJson(content)) {
                content = JSON.stringify(
                    JSON.parse(content), null, 2);
            }
        } catch (e) {
            msg.textContent =
                'Invalid config: ' + e.message;
            msg.className =
                'config-modal-msg error';
            return;
        }

        msg.textContent = 'Saving...';
        msg.className = 'config-modal-msg';

        const resp = await apiPost(
            'config/' + name, { content: content });

        if (resp && resp.status === 'ok') {
            rawEditor.value = content;
            activeConfigParsed =
                tryParseJson(content);
            msg.textContent =
                'Saved successfully!';
            msg.className =
                'config-modal-msg success';
            await loadConfigs();
            showToast(
                (CONFIG_LABELS[name] || name) +
                ' saved',
                'success'
            );
        } else {
            msg.textContent =
                (resp && resp.error) || 'Save failed';
            msg.className =
                'config-modal-msg error';
        }
    }

    document.getElementById('config-modal-close')
        .addEventListener('click',
            closeConfigModal);
    document.getElementById(
        'config-modal-backdrop')
        .addEventListener('click',
            closeConfigModal);
    document.getElementById(
        'config-tab-structured')
        .addEventListener('click', () => {
            setConfigModalMode('structured');
        });
    document.getElementById('config-tab-raw')
        .addEventListener('click', () => {
            setConfigModalMode('raw');
        });
    document.getElementById('config-modal-reload')
        .addEventListener('click', () => {
            if (activeConfigName) {
                openConfigModal(activeConfigName);
            }
        });
    document.getElementById('config-modal-save')
        .addEventListener('click', () => {
            if (activeConfigName) {
                saveConfig(activeConfigName);
            }
        });
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            closeConfigModal();
        }
    });

    // ==========================
    // OTA Updates
    // ==========================

    function loadOta() {
        const list =
            document.getElementById('ota-list');
        const status =
            document.getElementById('ota-status');
        list.innerHTML =
            '<p class="empty-state">' +
            'Click "Check for Updates" to scan ' +
            'for available skill updates.</p>';
        status.textContent = '';
    }

    document.getElementById('ota-check-btn')
        .addEventListener('click', async () => {
            const list =
                document.getElementById('ota-list');
            const status =
                document.getElementById('ota-status');
            status.textContent = 'Checking...';
            status.className = 'ota-status';
            list.innerHTML =
                '<p class="empty-state">' +
                'Scanning...</p>';

            const data =
                await apiFetch('ota/check');

            if (!data || data.error) {
                status.textContent =
                    data ? data.error : 'Failed';
                status.className =
                    'ota-status error';
                list.innerHTML =
                    '<p class="empty-state">' +
                    escHtml(
                        data ? data.error
                            : 'Check failed'
                    ) + '</p>';
                return;
            }

            const count = data.available_count || 0;
            status.textContent = count > 0
                ? count + ' update(s) available'
                : 'All skills up to date';
            status.className = count > 0
                ? 'ota-status warning'
                : 'ota-status success';

            if (!data.updates ||
                data.updates.length === 0) {
                list.innerHTML =
                    '<p class="empty-state">' +
                    'No skills in manifest</p>';
                return;
            }

            list.innerHTML = data.updates.map(
                u => {
                    const badge = u.update_available
                        ? '<span class="ota-badge ' +
                          'update">Update</span>'
                        : '<span class="ota-badge ' +
                          'current">Current</span>';
                    const actions =
                        u.update_available
                            ? '<button class="' +
                              'btn-outline ota-update"' +
                              ' data-skill="' +
                              escHtml(u.name) +
                              '">Update</button>'
                            : '';
                    return '<div class="card-item ' +
                        'ota-card">' +
                        '<div class="' +
                        'card-item-title">' +
                        escHtml(u.name) +
                        ' ' + badge + '</div>' +
                        '<div class="' +
                        'card-item-meta">' +
                        'Local: ' +
                        escHtml(u.local_version) +
                        ' → Remote: ' +
                        escHtml(u.remote_version) +
                        '</div>' +
                        '<div class="ota-actions">' +
                        actions +
                        '<button class="' +
                        'btn-outline ota-rollback"' +
                        ' data-skill="' +
                        escHtml(u.name) +
                        '">Rollback</button>' +
                        '</div></div>';
                }
            ).join('');

            // Bind update buttons
            list.querySelectorAll('.ota-update')
                .forEach(btn => {
                    btn.addEventListener(
                        'click', () => {
                            otaUpdateSkill(
                                btn.dataset.skill);
                        });
                });

            // Bind rollback buttons
            list.querySelectorAll('.ota-rollback')
                .forEach(btn => {
                    btn.addEventListener(
                        'click', () => {
                            otaRollbackSkill(
                                btn.dataset.skill);
                        });
                });
        });

    async function otaUpdateSkill(name) {
        const status =
            document.getElementById('ota-status');
        status.textContent =
            'Updating ' + name + '...';
        status.className = 'ota-status';

        const resp = await apiPost(
            'ota/update', { skill: name });

        if (resp && resp.status === 'updated') {
            status.textContent = name +
                ' updated to v' +
                resp.new_version;
            status.className =
                'ota-status success';
        } else if (
            resp && resp.status === 'up_to_date') {
            status.textContent = name +
                ' is already up to date';
            status.className =
                'ota-status success';
        } else {
            status.textContent =
                'Update failed: ' +
                (resp ? resp.error : 'unknown');
            status.className = 'ota-status error';
        }
    }

    async function otaRollbackSkill(name) {
        if (!confirm(
            'Rollback ' + name +
            ' to previous version?')) return;

        const status =
            document.getElementById('ota-status');
        status.textContent =
            'Rolling back ' + name + '...';
        status.className = 'ota-status';

        const resp = await apiPost(
            'ota/rollback', { skill: name });

        if (resp &&
            resp.status === 'rolled_back') {
            status.textContent = name +
                ' rolled back to v' +
                resp.restored_version;
            status.className =
                'ota-status success';
        } else {
            status.textContent =
                'Rollback failed: ' +
                (resp ? resp.error : 'unknown');
            status.className = 'ota-status error';
        }
    }

    // --- Utility ---
    function escHtml(s) {
        const div = document.createElement('div');
        div.textContent = s;
        return div.innerHTML;
    }

    // --- Toast Notifications ---
    function showToast(msg, type, durationMs) {
        durationMs = durationMs || 3000;
        const container =
            document.getElementById('toast-container');
        if (!container) return;
        const el = document.createElement('div');
        el.className = 'toast' + (type ? ' ' + type : '');
        el.textContent = msg;
        container.appendChild(el);
        setTimeout(function () {
            el.style.animation =
                'toastOut 0.22s ease forwards';
            setTimeout(function () {
                el.remove();
            }, 230);
        }, durationMs);
    }
    window._showToast = showToast;

    // --- Hook Approval Modal ---
    let currentApprovalId = null;

    function showHookApprovalModal(req) {
        currentApprovalId = req.approval_id;
        const modal = document.getElementById('hook-approval-modal');
        const toolNameEl = document.getElementById('approval-tool-name');
        const toolArgsEl = document.getElementById('approval-tool-args');
        const dangerBadge = document.getElementById('approval-danger-badge');
        const countdownText = document.getElementById('approval-countdown-text');
        const progressCircle = document.getElementById('countdown-ring-progress');

        if (!modal) return;

        if (toolNameEl) toolNameEl.textContent = req.tool;
        if (toolArgsEl) {
            toolArgsEl.textContent = typeof req.arguments === 'object'
                ? JSON.stringify(req.arguments, null, 2)
                : String(req.arguments);
        }

        if (dangerBadge) {
            const isDanger = req.tool === 'run_command' || req.tool.startsWith('run_') || req.tool.includes('execute');
            dangerBadge.style.display = isDanger ? '' : 'none';
        }

        modal.classList.remove('hidden');
        document.body.classList.add('modal-open');

        let totalSeconds = 30;
        let remainingSeconds = totalSeconds;
        if (countdownText) countdownText.textContent = remainingSeconds;
        if (progressCircle) {
            progressCircle.style.strokeDashoffset = '0';
        }

        if (approvalCountdownTimer) {
            clearInterval(approvalCountdownTimer);
        }

        approvalCountdownTimer = setInterval(() => {
            remainingSeconds--;
            if (countdownText) countdownText.textContent = remainingSeconds;

            if (progressCircle) {
                const dashArray = 213.6;
                const offset = dashArray * (1 - remainingSeconds / totalSeconds);
                progressCircle.style.strokeDashoffset = offset;
            }

            if (remainingSeconds <= 0) {
                clearInterval(approvalCountdownTimer);
                hideHookApprovalModal();
            }
        }, 1000);
    }

    function hideHookApprovalModal() {
        const modal = document.getElementById('hook-approval-modal');
        if (modal) {
            modal.classList.add('hidden');
        }
        document.body.classList.remove('modal-open');
        if (approvalCountdownTimer) {
            clearInterval(approvalCountdownTimer);
            approvalCountdownTimer = null;
        }
        currentApprovalId = null;
    }

    async function submitApprovalDecision(allowed) {
        if (!currentApprovalId) return;
        const approvalId = currentApprovalId;
        hideHookApprovalModal();

        const resp = await apiPost('approval', {
            approval_id: approvalId,
            allowed: allowed
        });

        if (resp && resp.status === 'ok') {
            showToast(allowed ? 'Action approved' : 'Action denied', allowed ? 'success' : 'warning');
        } else {
            showToast('Failed to submit decision: ' + (resp ? resp.error : 'unknown'), 'error');
        }
    }

    function checkAndShowHookApproval(text) {
        if (!text) return text;
        const match = text.match(/<approval_request>([\s\S]*?)<\/approval_request>/);
        if (match) {
            try {
                const req = JSON.parse(match[1]);
                showHookApprovalModal(req);
            } catch (e) {
                console.error("Failed to parse approval request JSON:", e);
            }
            return text.replace(/<approval_request>[\s\S]*?<\/approval_request>/, '').trim();
        }
        return text;
    }

    // Bind approval action buttons
    const btnApproveAction = document.getElementById('btn-approve-action');
    const btnDenyAction = document.getElementById('btn-deny-action');
    if (btnApproveAction) {
        btnApproveAction.addEventListener('click', () => {
            submitApprovalDecision(true);
        });
    }
    if (btnDenyAction) {
        btnDenyAction.addEventListener('click', () => {
            submitApprovalDecision(false);
        });
    }

    // --- Execution Timeline ---
    let timelineNodes = [];

    function clearTimeline() {
        timelineNodes = [];
        const container = document.getElementById('timeline-container');
        if (container) {
            container.innerHTML = '<div class="timeline-empty-hint">No tool execution logs in this turn yet.</div>';
        }
    }

    function addTimelineNode(tool, args) {
        const container = document.getElementById('timeline-container');
        if (!container) return;

        let listEl = container.querySelector('.timeline-list');
        if (!listEl) {
            container.innerHTML = '<div class="timeline-list"></div>';
            listEl = container.querySelector('.timeline-list');
        }

        const id = 'tl-' + Date.now() + '-' + Math.random().toString(36).substr(2, 5);
        const node = { id, tool, state: 'running', timestamp: new Date() };
        timelineNodes.push(node);

        const argsStr = args ? JSON.stringify(args) : '';

        const nodeEl = document.createElement('div');
        nodeEl.className = 'timeline-node';
        nodeEl.id = id;
        nodeEl.innerHTML = `
            <div class="timeline-node-dot running"></div>
            <div class="timeline-node-title">${escHtml(tool)}</div>
            <div class="timeline-node-subtitle">${escHtml(argsStr)}</div>
        `;
        listEl.appendChild(nodeEl);
        container.scrollTop = container.scrollHeight;

        const sidebar = document.getElementById('chat-timeline-sidebar');
        if (sidebar && sidebar.classList.contains('collapsed')) {
            sidebar.classList.remove('collapsed');
        }
    }

    function updateTimelineNode(tool, success, error) {
        const node = [...timelineNodes].reverse().find(n => n.tool === tool && n.state === 'running');
        if (!node) return;

        node.state = success ? 'success' : 'failure';
        const el = document.getElementById(node.id);
        if (el) {
            const dot = el.querySelector('.timeline-node-dot');
            if (dot) {
                dot.className = 'timeline-node-dot ' + (success ? 'success' : 'failure');
            }
            if (error) {
                const sub = el.querySelector('.timeline-node-subtitle');
                if (sub) {
                    sub.textContent += ' (Error: ' + error + ')';
                }
            }
        }
    }

    // Timeline Sidebar toggle handlers
    const toggleTimelineBtn = document.getElementById('toggle-timeline-btn');
    const chatTimelineSidebar = document.getElementById('chat-timeline-sidebar');
    const closeTimelineBtn = document.getElementById('close-timeline-btn');

    if (toggleTimelineBtn && chatTimelineSidebar) {
        toggleTimelineBtn.addEventListener('click', () => {
            chatTimelineSidebar.classList.toggle('collapsed');
        });
    }
    if (closeTimelineBtn && chatTimelineSidebar) {
        closeTimelineBtn.addEventListener('click', () => {
            chatTimelineSidebar.classList.add('collapsed');
        });
    }

    // --- SSE Event Stream ---
    function initEventStream() {
        if (eventSource) {
            eventSource.close();
            eventSource = null;
        }

        const token = authToken || '';
        const url = `/api/events?token=${encodeURIComponent(token)}`;

        eventSource = new EventSource(url);

        eventSource.onmessage = function(event) {
            try {
                const raw = JSON.parse(event.data);
                const type = parseEventType(raw.event_type || '');
                handleIncomingEvent(type, raw.data, raw.timestamp);
            } catch (e) {
                console.error("Failed to parse event:", e);
            }
        };

        eventSource.onerror = function(err) {
            console.error("SSE connection error:", err);
        };
    }

    function parseEventType(typeStr) {
        if (typeStr.startsWith('Custom("') && typeStr.endsWith('")')) {
            return typeStr.substring(8, typeStr.length - 2);
        }
        return typeStr;
    }

    function handleIncomingEvent(type, data, timestamp) {
        if (type === 'tool_start') {
            addTimelineNode(data.tool, data.arguments);
        } else if (type === 'tool_end') {
            updateTimelineNode(data.tool, data.success, data.error);
        } else if (type === 'hook_approval_request') {
            showHookApprovalModal(data);
        } else if (type === 'hook_approval_resolved') {
            if (currentApprovalId === data.approval_id) {
                hideHookApprovalModal();
            }
        }
    }

    // --- Skills Catalog ---
    async function loadSkills() {
        const grid = document.getElementById('skills-grid');
        const draftsContainer = document.getElementById('skills-drafts-container');
        const draftsList = document.getElementById('skills-drafts-list');

        if (grid) {
            grid.innerHTML = `
                <div class="skeleton-card shimmer"></div>
                <div class="skeleton-card shimmer"></div>
                <div class="skeleton-card shimmer"></div>
            `;
        }

        const data = await apiFetch('skills');
        if (!data || !grid) {
            if (grid) grid.innerHTML = '<p class="empty-state">Failed to load skills.</p>';
            return;
        }

        const drafts = data.drafts || [];
        if (drafts.length > 0 && draftsContainer && draftsList) {
            draftsContainer.classList.remove('hidden');
            draftsList.innerHTML = drafts.map(draft => {
                const diffHtml = generateDiffHtml(draft.original_content, draft.content);
                return `
                    <div class="draft-card" data-draft-name="${escHtml(draft.name)}">
                        <div class="draft-card-header">
                            <div>
                                <span class="draft-badge">Draft Skill</span>
                                <h4 style="margin: 4px 0 0 0; font-size: 0.95rem; color: var(--text-primary);">${escHtml(draft.name)}</h4>
                            </div>
                            <span class="skill-type">Markdown Draft</span>
                        </div>
                        <div class="draft-diff-container">
                            <div class="draft-diff-header">SKILL.md.draft</div>
                            <div class="draft-diff-body">${diffHtml}</div>
                        </div>
                        <div class="draft-card-actions">
                            <button class="btn-outline btn-danger-outline btn-discard-draft">Discard Draft</button>
                            <button class="btn-primary btn-success btn-approve-draft">Approve & Save</button>
                        </div>
                    </div>
                `;
            }).join('');

            draftsList.querySelectorAll('.draft-card').forEach(card => {
                const name = card.dataset.draftName;
                card.querySelector('.btn-approve-draft').addEventListener('click', async () => {
                    card.querySelector('.btn-approve-draft').disabled = true;
                    const resp = await apiPost('skills/approve', { name });
                    if (resp && resp.status === 'ok') {
                        showToast(`Approved and saved skill: ${name}`, 'success');
                        loadSkills();
                    } else {
                        showToast(`Failed to approve: ${resp ? resp.error : 'unknown'}`, 'error');
                        card.querySelector('.btn-approve-draft').disabled = false;
                    }
                });
                card.querySelector('.btn-discard-draft').addEventListener('click', async () => {
                    if (!confirm(`Are you sure you want to discard draft for ${name}?`)) return;
                    card.querySelector('.btn-discard-draft').disabled = true;
                    const resp = await apiPost('skills/discard', { name });
                    if (resp && resp.status === 'ok') {
                        showToast(`Discarded draft: ${name}`, 'success');
                        loadSkills();
                    } else {
                        showToast(`Failed to discard: ${resp ? resp.error : 'unknown'}`, 'error');
                        card.querySelector('.btn-discard-draft').disabled = false;
                    }
                });
            });
        } else if (draftsContainer) {
            draftsContainer.classList.add('hidden');
        }

        const skills = data.skills || [];
        if (skills.length === 0) {
            grid.innerHTML = '<p class="empty-state">No skills loaded.</p>';
            return;
        }

        grid.innerHTML = skills.map(skill => {
            const checked = skill.enabled ? 'checked' : '';
            const missingDeps = !skill.dependency_ready && skill.missing_requires && skill.missing_requires.length > 0
                ? `<div class="skill-path" style="border-color: rgba(239, 68, 68, 0.2); color: var(--danger); margin-top: 6px;">Missing dependencies: ${escHtml(skill.missing_requires.join(', '))}</div>`
                : '';
            return `
                <div class="skill-card">
                    <div class="skill-card-header">
                        <div class="skill-title-wrap">
                            <h4 class="skill-title" title="${escHtml(skill.name)}">${escHtml(skill.name)}</h4>
                            <span class="skill-type">${escHtml(skill.root_kind || 'user')} skill</span>
                        </div>
                        <div class="skill-toggle-wrap">
                            <label class="switch">
                                <input type="checkbox" class="skill-toggle-input" data-skill-name="${escHtml(skill.name)}" ${checked}>
                                <span class="slider"></span>
                            </label>
                        </div>
                    </div>
                    <p class="skill-desc">${escHtml(skill.description || 'No description provided.')}</p>
                    <div class="skill-path" title="${escHtml(skill.path)}">${escHtml(skill.path)}</div>
                    ${missingDeps}
                </div>
            `;
        }).join('');

        grid.querySelectorAll('.skill-toggle-input').forEach(input => {
            input.addEventListener('change', async () => {
                const name = input.dataset.skillName;
                const enabled = input.checked;
                input.disabled = true;
                const resp = await apiPost('skills', { name, enabled });
                if (resp && !resp.error) {
                    showToast(`Skill ${name} ${enabled ? 'enabled' : 'disabled'}`, 'success');
                } else {
                    showToast(`Failed to toggle skill: ${resp ? resp.error : 'unknown'}`, 'error');
                    input.checked = !enabled;
                }
                input.disabled = false;
            });
        });
    }

    function generateDiffHtml(original, draft) {
        if (!original) {
            return draft.split('\n').map(l => `<div class="diff-line added">+ ${escHtml(l)}</div>`).join('');
        }
        const origLines = original.split('\n');
        const draftLines = draft.split('\n');

        let html = '';
        const max = Math.max(origLines.length, draftLines.length);
        for (let i = 0; i < max; i++) {
            const orig = origLines[i];
            const drft = draftLines[i];
            if (orig === drft) {
                if (orig !== undefined) {
                    html += `<div class="diff-line normal">  ${escHtml(orig)}</div>`;
                }
            } else {
                if (orig !== undefined) {
                    html += `<div class="diff-line deleted">- ${escHtml(orig)}</div>`;
                }
                if (drft !== undefined) {
                    html += `<div class="diff-line added">+ ${escHtml(drft)}</div>`;
                }
            }
        }
        return html;
    }

    // --- Live Trend Sparklines (canvas, no deps) ---
    const CHART_MAX = 60;
    const chartHistory = { cpu: [], mem: [], calls: [], tokens: [] };

    function pushSample(key, value) {
        if (value == null || isNaN(value)) return;
        const arr = chartHistory[key];
        if (!arr) return;
        arr.push(value);
        if (arr.length > CHART_MAX) arr.shift();
    }

    function drawSparkline(canvasId, data, color) {
        const canvas = document.getElementById(canvasId);
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (!ctx) return;
        const dpr = window.devicePixelRatio || 1;
        const cssW = canvas.clientWidth || 300;
        const cssH = canvas.clientHeight || 64;
        if (canvas.width !== Math.round(cssW * dpr) || canvas.height !== Math.round(cssH * dpr)) {
            canvas.width = Math.round(cssW * dpr);
            canvas.height = Math.round(cssH * dpr);
        }
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, cssW, cssH);
        if (!data || data.length < 2) return;
        const pad = 4;
        const w = cssW - pad * 2;
        const h = cssH - pad * 2;
        let min = Math.min.apply(null, data);
        let max = Math.max.apply(null, data);
        if (max - min < 1e-9) max = min + 1;
        const xAt = i => pad + (i / (data.length - 1)) * w;
        const yAt = v => pad + h - ((v - min) / (max - min)) * h;
        ctx.beginPath();
        ctx.moveTo(xAt(0), yAt(data[0]));
        for (let i = 1; i < data.length; i++) ctx.lineTo(xAt(i), yAt(data[i]));
        ctx.lineTo(xAt(data.length - 1), pad + h);
        ctx.lineTo(xAt(0), pad + h);
        ctx.closePath();
        ctx.fillStyle = color + '22';
        ctx.fill();
        ctx.beginPath();
        ctx.moveTo(xAt(0), yAt(data[0]));
        for (let i = 1; i < data.length; i++) ctx.lineTo(xAt(i), yAt(data[i]));
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.lineJoin = 'round';
        ctx.stroke();
        ctx.beginPath();
        ctx.arc(xAt(data.length - 1), yAt(data[data.length - 1]), 2.5, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();
    }

    function chartColors() {
        const css = getComputedStyle(document.documentElement);
        const get = (n, d) => (css.getPropertyValue(n) || d).trim() || d;
        return {
            cpu: get('--c-teal', '#2dd4bf'),
            mem: get('--accent-light', '#818cf8'),
            calls: get('--c-amber', '#fbbf24'),
            tokens: get('--c-violet', '#c084fc'),
        };
    }

    function drawAllCharts() {
        const c = chartColors();
        drawSparkline('chart-cpu', chartHistory.cpu, c.cpu);
        drawSparkline('chart-mem', chartHistory.mem, c.mem);
        drawSparkline('chart-calls', chartHistory.calls, c.calls);
        drawSparkline('chart-tokens', chartHistory.tokens, c.tokens);
    }
    // Allow the theme toggle to redraw with theme-aware colors.
    window.__voxiRedrawCharts = drawAllCharts;

    function updateCharts(m) {
        if (!m) return;
        const cpu = (m.cpu && m.cpu.load_1m != null) ? m.cpu.load_1m : null;
        const mem = (m.memory && m.memory.vm_rss_kb != null) ? m.memory.vm_rss_kb / 1024 : null;
        const calls = (m.counters && m.counters.llm_calls != null) ? m.counters.llm_calls : null;
        const tokens = m.tokens ? ((m.tokens.prompt || 0) + (m.tokens.completion || 0)) : null;
        pushSample('cpu', cpu);
        pushSample('mem', mem);
        pushSample('calls', calls);
        pushSample('tokens', tokens);
        drawAllCharts();
        const setNow = (id, v) => { const el = document.getElementById(id); if (el) el.textContent = v; };
        setNow('trend-cpu-now', cpu != null ? cpu.toFixed(2) : '—');
        setNow('trend-mem-now', mem != null ? mem.toFixed(0) + ' MB' : '—');
        setNow('trend-calls-now', calls != null ? calls : '—');
        setNow('trend-tokens-now', tokens != null ? fmtTokens(tokens) : '—');
    }

    // --- Theme toggle (light / dark) ---
    (function initTheme() {
        const saved = localStorage.getItem('voxi-theme');
        if (saved === 'light' || saved === 'dark') {
            document.documentElement.setAttribute('data-theme', saved);
        }
        const toggle = document.getElementById('theme-toggle');
        if (toggle) {
            toggle.addEventListener('click', function () {
                const cur = document.documentElement.getAttribute('data-theme') === 'light' ? 'light' : 'dark';
                const next = cur === 'light' ? 'dark' : 'light';
                document.documentElement.setAttribute('data-theme', next);
                localStorage.setItem('voxi-theme', next);
                if (typeof window.__voxiRedrawCharts === 'function') window.__voxiRedrawCharts();
            });
        }
    })();

    // --- Initial Load ---
    formatChatSessionMeta();
    startOutboundPolling();
    initEventStream();
    loadDashboard();

    // Wire up event delegation for log tail buttons once
    const logContentEl = document.getElementById('log-content');
    if (logContentEl) {
        logContentEl.addEventListener('click', function(ev) {
            const btn = ev.target.closest('.log-tail-btn');
            if (btn) {
                const lbl = btn.dataset.label;
                const pre = logContentEl.querySelector('.log-pre[data-label="' + lbl + '"]');
                if (pre) {
                    pre.scrollTop = pre.scrollHeight;
                }
            }
        });
    }

    // --- Semantic Search (ONNX-backed) ---
    function focusSearch() {
        const input = document.getElementById('search-input');
        if (input) { input.focus(); input.select(); }
    }

    async function runSearch() {
        const input = document.getElementById('search-input');
        const resultsEl = document.getElementById('search-results');
        const hintEl = document.getElementById('search-hint');
        if (!input || !resultsEl) return;
        const q = input.value.trim();
        if (!q) { resultsEl.innerHTML = ''; if (hintEl) hintEl.textContent = ''; return; }
        resultsEl.innerHTML =
            '<div class="skeleton-card shimmer"></div>' +
            '<div class="skeleton-card shimmer"></div>' +
            '<div class="skeleton-card shimmer"></div>';
        if (hintEl) hintEl.textContent = 'Searching…';
        const data = await apiFetch('search?q=' + encodeURIComponent(q) + '&top_k=12');
        const results = (data && Array.isArray(data.results)) ? data.results : [];
        renderSearchResults(results, q);
    }

    function renderSearchResults(results, q) {
        const resultsEl = document.getElementById('search-results');
        const hintEl = document.getElementById('search-hint');
        if (!resultsEl) return;
        if (!results.length) {
            resultsEl.innerHTML =
                '<p class="empty-state">No semantically similar memory found for &ldquo;' +
                escHtml(q) + '&rdquo;.</p>';
            if (hintEl) hintEl.textContent =
                'Tip: semantic search matches meaning — try a natural phrase.';
            return;
        }
        if (hintEl) hintEl.textContent =
            results.length + ' result' + (results.length === 1 ? '' : 's') +
            ' ranked by semantic similarity.';
        resultsEl.innerHTML = results.map(function (r) {
            const score = Math.max(0, Math.min(1, Number(r.score) || 0));
            const pct = Math.round(score * 100);
            return '<div class="search-result">' +
                '<div class="search-result-head">' +
                '<span class="search-result-key">' + escHtml(r.key || '(untitled)') + '</span>' +
                (r.category ? '<span class="search-result-cat">' + escHtml(r.category) + '</span>' : '') +
                '<span class="search-score" title="cosine similarity">' + pct + '%</span>' +
                '</div>' +
                '<div class="search-result-snippet">' + escHtml(r.snippet || '') + '</div>' +
                '</div>';
        }).join('');
    }

    (function bindSearch() {
        const btn = document.getElementById('search-btn');
        const input = document.getElementById('search-input');
        if (btn) btn.addEventListener('click', runSearch);
        if (input) {
            input.addEventListener('keydown', function (e) {
                if (e.key === 'Enter') { e.preventDefault(); runSearch(); }
            });
        }
        // Global Ctrl/Cmd+K opens semantic search.
        document.addEventListener('keydown', function (e) {
            if ((e.metaKey || e.ctrlKey) && (e.key === 'k' || e.key === 'K')) {
                e.preventDefault();
                navigateTo('search');
                focusSearch();
            }
        });
    })();
})();
