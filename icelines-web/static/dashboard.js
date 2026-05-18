(function () {
    var commandHistory = readCommandHistory();
    var commandHistoryIndex = commandHistory.length;

    function dashboardWorkspaceFromUrl(url) {
        var parsed = parseUrl(url);
        if (!parsed) return null;
        return parsed.pathname === "/dashboard" ? parsed.searchParams.get("workspace") : null;
    }

    function parseUrl(url) {
        try {
            return new URL(url, window.location.origin);
        } catch (_) {
            return null;
        }
    }

    function isSameOriginUrl(parsed) {
        return parsed.origin === window.location.origin;
    }

    function isIgnoredAppPath(pathname) {
        return pathname.indexOf("/api/") === 0 ||
            pathname.indexOf("/static/") === 0 ||
            pathname.indexOf("/season-type/") === 0 ||
            pathname === "/seasons";
    }

    function appWorkspaceFromUrl(url) {
        var parsed = parseUrl(url);
        if (!parsed) return null;
        return appWorkspaceFromParsed(parsed, url);
    }

    function appWorkspaceFromParsed(parsed, url) {
        if (!isSameOriginUrl(parsed)) return null;
        if (parsed.pathname === "/dashboard") return dashboardWorkspaceFromUrl(url);
        if (isIgnoredAppPath(parsed.pathname)) return null;
        var workspace = parsed.pathname + parsed.search;
        return dashboardWorkspaceOrNull(workspace);
    }

    function dashboardWorkspaceOrNull(workspace) {
        if (isDashboardWorkspace(workspace)) return workspace;
        return null;
    }

    function isDashboardWorkspace(workspace) {
        var path = String(workspace || "").split("?")[0].replace(/\/+$/, "") || "/";
        var patterns = [/^\/player\/[0-9]+$/, /^\/team\/[A-Za-z]{2,3}(\/season)?$/, /^\/game\/[^/]+$/];
        var knownPaths = [
            "/", "/leaders", "/goalies", "/depth", "/poach", "/fantasy",
            "/scores", "/schedule", "/transactions", "/playoffs",
            "/favorites", "/watchlist", "/career", "/reports/poach",
            "/reports/weekly", "/admin", "/docs"
        ];
        return patterns.some(function (pattern) { return pattern.test(path); }) || knownPaths.indexOf(path) !== -1;
    }

    function copyDashboardState(url) {
        ["left", "right", "experience", "left_workspace", "right_workspace"].forEach(function (key) {
            var value = new URLSearchParams(window.location.search).get(key);
            if (value) url.searchParams.set(key, value);
        });
    }

    function dashboardUrl(workspace) {
        var url = new URL("/dashboard", window.location.origin);
        url.searchParams.set("workspace", workspace || "/leaders");
        copyDashboardState(url);
        return url;
    }

    function currentWorkspace() {
        var input = document.querySelector("[data-dashboard-workspace-input]");
        if (input && input.value) return input.value;
        return new URLSearchParams(window.location.search).get("workspace") || "/leaders";
    }

    function paneTargetFromClick(event) {
        if ([event.metaKey, event.altKey, !event.ctrlKey].some(Boolean)) return null;
        if (event.shiftKey) return "right";
        return "left";
    }

    function paneTargetUrl(workspace, pane) {
        var url = dashboardUrl(currentWorkspace());
        url.searchParams.set(pane + "_workspace", workspace || "/leaders");
        return url;
    }

    function compositionUrl(href) {
        var url = new URL(href, window.location.origin);
        if (url.origin !== window.location.origin || url.pathname !== "/dashboard") return null;
        ["left_workspace", "right_workspace"].forEach(function (key) {
            var value = new URLSearchParams(window.location.search).get(key);
            if (value && !url.searchParams.has(key)) url.searchParams.set(key, value);
        });
        return url;
    }

    function followDashboardComposition(href) {
        var composedUrl = compositionUrl(href);
        window.location.href = (composedUrl || new URL(href, window.location.origin)).toString();
    }

    function partialUrl(workspace) {
        var url = dashboardUrl(workspace);
        url.searchParams.set("partial", "workspace");
        return url;
    }

    async function loadWorkspace(workspace, push) {
        var current = document.querySelector(".jaw-workspace");
        if (!current) return false;

        var response = await fetch(partialUrl(workspace).toString(), {
            headers: { "X-Requested-With": "fetch" }
        });
        if (!response.ok) return false;

        return replaceWorkspaceFromResponse(current, workspace, push, await response.text());
    }

    function replaceWorkspaceFromResponse(current, workspace, push, html) {
        var template = document.createElement("template");
        template.innerHTML = html.trim();
        var next = template.content.querySelector(".jaw-workspace");
        if (!next) return false;

        current.replaceWith(next);
        var nextWorkspace = next.getAttribute("data-workspace-url") || workspace;
        updateCommandWorkspace(nextWorkspace);
        pushWorkspaceState(push, nextWorkspace);
        return true;
    }

    function pushWorkspaceState(push, nextWorkspace) {
        if (!push) return;
        window.history.pushState(
            { workspace: nextWorkspace },
            "",
            dashboardUrl(nextWorkspace).toString()
        );
    }

    function commandStatusText(message, kind) {
        var text = message || "";
        if (kind !== "error") return text;
        return errorStatusText(text);
    }

    function errorStatusText(text) {
        if (!text) return text;
        if (/^error[:\s]/i.test(text)) return text;
        return "Error: " + text;
    }

    function setCommandStatus(message, kind) {
        var node = document.querySelector("[data-dashboard-command-status]");
        if (!node) return;
        node.textContent = commandStatusText(message, kind);
        node.dataset.statusKind = kind || "";
    }

    function commandInput() {
        return document.querySelector("[data-dashboard-command-input]");
    }

    function updateCommandWorkspace(workspace) {
        var input = document.querySelector("[data-dashboard-workspace-input]");
        if (input) input.value = workspace || "/leaders";
    }

    function focusCommandInput() {
        var input = commandInput();
        if (!input) return;
        input.focus();
        input.select();
    }

    function isEditableTarget(target) {
        if (!target) return false;
        var tag = String(target.tagName || "").toLowerCase();
        return ["input", "textarea", "select"].indexOf(tag) !== -1 || target.isContentEditable;
    }

    function commandHistoryKey() {
        return "icelines.dashboard.command.history";
    }

    function readCommandHistory() {
        try {
            var raw = window.sessionStorage.getItem(commandHistoryKey());
            var parsed = raw && JSON.parse(raw);
            return Array.isArray(parsed) ? parsed.filter(Boolean).slice(-25) : [];
        } catch (_) {
            return [];
        }
    }

    function writeCommandHistory() {
        try {
            window.sessionStorage.setItem(commandHistoryKey(), JSON.stringify(commandHistory.slice(-25)));
        } catch (_) {}
    }

    function pushCommandHistory(command) {
        var trimmed = String(command || "").trim();
        if (!trimmed) return;
        if (commandHistory[commandHistory.length - 1] !== trimmed) {
            commandHistory.push(trimmed);
        }
        commandHistory = commandHistory.slice(-25);
        commandHistoryIndex = commandHistory.length;
        writeCommandHistory();
    }

    function recallCommandHistory(input, direction) {
        if (!commandHistory.length) return;
        commandHistoryIndex = Math.min(Math.max(commandHistoryIndex + direction, 0), commandHistory.length);
        input.value = commandHistory[commandHistoryIndex] || "";
    }

    function paneStorageKey(pane) {
        return "icelines.dashboard.pane." + pane;
    }

    function panePreference(pane) {
        try {
            return window.localStorage.getItem(paneStorageKey(pane));
        } catch (_) {
            return null;
        }
    }

    function updatePaneNode(node, visible) {
        if (node) node.setAttribute("data-dashboard-pane-collapsed", visible ? "false" : "true");
    }

    function updatePaneToggle(toggle, visible) {
        if (!toggle) return;
        toggle.setAttribute("aria-expanded", visible ? "true" : "false");
        toggle.textContent = visible ? "Hide" : "Show";
    }

    function persistPanePreference(pane, visible) {
        try {
            window.localStorage.setItem(paneStorageKey(pane), visible ? "show" : "hide");
        } catch (_) {}
    }

    function setPaneVisible(pane, visible, persist) {
        var node = document.querySelector("[data-dashboard-pane='" + pane + "']");
        var toggle = document.querySelector("[data-dashboard-pane-toggle='" + pane + "']");
        updatePaneNode(node, visible);
        updatePaneToggle(toggle, visible);
        if (!node) return;
        if (persist === false) return;
        persistPanePreference(pane, visible);
    }

    function restorePanes() {
        ["favorites", "schedule"].forEach(function (pane) {
            if (panePreference(pane) === "hide") {
                setPaneVisible(pane, false, false);
            }
        });
        if (window.matchMedia && window.matchMedia("(max-width: 980px)").matches && !panePreference("schedule")) {
            setPaneVisible("schedule", false, false);
        }
    }

    function applyCommandSideEffect(command) {
        var normalized = (command || "").trim().toLowerCase();
        var actions = {
            "/hide favorites": ["favorites", false],
            "/hide fav": ["favorites", false],
            "/show favorites": ["favorites", true],
            "/show fav": ["favorites", true],
            "/hide schedule": ["schedule", false],
            "/hide sched": ["schedule", false],
            "/show schedule": ["schedule", true],
            "/show sched": ["schedule", true]
        };
        var action = actions[normalized];
        if (action) {
            setPaneVisible(action[0], action[1]);
        }
    }

    function isPlainPrimaryClick(event) {
        return [
            !event.defaultPrevented,
            event.button === 0,
            !event.metaKey,
            !event.ctrlKey,
            !event.altKey
        ].every(Boolean);
    }

    function isCompositionClick(event, link) {
        return link.hasAttribute("data-dashboard-composition-link") &&
            isPlainPrimaryClick(event) &&
            !event.shiftKey &&
            compositionUrl(link.href);
    }

    function handlePaneToggleClick(event) {
        var toggle = event.target.closest("[data-dashboard-pane-toggle]");
        if (!toggle) return false;
        event.preventDefault();
        var pane = toggle.getAttribute("data-dashboard-pane-toggle");
        var node = document.querySelector("[data-dashboard-pane='" + pane + "']");
        if (node) setPaneVisible(pane, node.getAttribute("data-dashboard-pane-collapsed") === "true");
        return true;
    }

    function handleDashboardLinkClick(event) {
        var link = event.target.closest("a[href]");
        if (!link) return false;
        if (handleCompositionLinkClick(event, link)) return true;
        if (!isPlainPrimaryClick(event)) return false;
        return handleWorkspaceLinkClick(event, link);
    }

    function handleCompositionLinkClick(event, link) {
        if (!isCompositionClick(event, link)) return false;
        event.preventDefault();
        followDashboardComposition(link.href);
        return true;
    }

    function handleWorkspaceLinkClick(event, link) {
        var paneTarget = linkPaneTarget(event, link);
        if (isShiftWithoutPane(event, paneTarget)) return false;

        var workspace = linkWorkspace(link);
        if (!workspace) return false;

        event.preventDefault();
        return navigateWorkspaceLink(link, workspace, paneTarget);
    }

    function linkPaneTarget(event, link) {
        return link.getAttribute("data-dashboard-target") || paneTargetFromClick(event);
    }

    function isShiftWithoutPane(event, paneTarget) {
        return !paneTarget && event.shiftKey;
    }

    function linkWorkspace(link) {
        return link.getAttribute("data-dashboard-workspace") || appWorkspaceFromUrl(link.href);
    }

    function navigateWorkspaceLink(link, workspace, paneTarget) {
        if ({ left: true, right: true }[paneTarget]) {
            window.location.href = paneTargetUrl(workspace, paneTarget).toString();
            return true;
        }
        loadWorkspace(workspace, true)
            .then(function (loaded) {
                if (!loaded) window.location.href = link.href;
            })
            .catch(function () {
                window.location.href = link.href;
            });
        return true;
    }

    document.addEventListener("click", function (event) {
        if (handlePaneToggleClick(event)) return;
        handleDashboardLinkClick(event);
    });

    window.addEventListener("popstate", function () {
        var params = new URLSearchParams(window.location.search);
        var workspace = params.get("workspace") || "/leaders";
        loadWorkspace(workspace, false).catch(function () {
            window.location.href = dashboardUrl(workspace).toString();
        });
    });

    function followShortcutSelector(event, selector) {
        var link = document.querySelector(selector);
        if (!link) return false;
        event.preventDefault();
        followDashboardComposition(link.href);
        return true;
    }

    function handleGlobalShortcut(event) {
        if (handleCommandFocusShortcut(event)) return true;
        if (isEditableTarget(event.target)) return false;
        return handleNavigationShortcut(event);
    }

    function focusCommandShortcut(event) {
        event.preventDefault();
        focusCommandInput();
        return true;
    }

    function handleCommandFocusShortcut(event) {
        if (handleSlashFocusShortcut(event)) return true;
        return handleKeyboardFocusShortcut(event);
    }

    function handleSlashFocusShortcut(event) {
        if (event.key !== "/") return false;
        if (isEditableTarget(event.target)) return false;
        event.preventDefault();
        focusCommandInput();
        return true;
    }

    function handleKeyboardFocusShortcut(event) {
        if (event.key.toLowerCase() !== "k") return false;
        if (event.ctrlKey) return focusCommandShortcut(event);
        if (event.metaKey) return focusCommandShortcut(event);
        return false;
    }

    function handleNavigationShortcut(event) {
        var selectors = {
            "]": "[data-dashboard-ring='right']",
            "[": "[data-dashboard-ring='left']",
            "\\": "[data-dashboard-pane-swap='right'], [data-dashboard-pane-swap='left']"
        };
        var selector = selectors[event.key];
        return selector ? followShortcutSelector(event, selector) : false;
    }

    function handleCommandInputKey(event) {
        if (event.target !== commandInput()) return false;
        if (event.key === "Escape") {
            event.preventDefault();
            event.target.value = "";
            setCommandStatus("", "");
            return true;
        }
        var historyDirection = { ArrowUp: -1, ArrowDown: 1 }[event.key];
        if (historyDirection) {
            event.preventDefault();
            recallCommandHistory(event.target, historyDirection);
            return true;
        }
        return false;
    }

    document.addEventListener("keydown", function (event) {
        if (handleGlobalShortcut(event)) return;
        handleCommandInputKey(event);
    });

    function workspaceFromForm(form) {
        var formUrl = new URL(form.action || window.location.href, window.location.origin);
        var params = new URLSearchParams(new FormData(form));
        var query = params.toString();
        return formUrl.pathname + (query ? "?" + query : "");
    }

    function handleWorkspaceFormSubmit(event, form) {
        if (!isWorkspaceGetForm(form)) return false;
        event.preventDefault();
        var workspace = workspaceFromForm(form);
        return submitWorkspaceFormWorkspace(workspace);
    }

    function isWorkspaceGetForm(form) {
        if (!form.closest(".jaw-workspace")) return false;
        return String(form.method || "get").toLowerCase() === "get";
    }

    function submitWorkspaceFormWorkspace(workspace) {
        if (!isDashboardWorkspace(workspace)) {
            window.location.href = workspace;
            return true;
        }
        loadWorkspace(workspace, true)
            .then(function (loaded) {
                if (!loaded) window.location.href = workspace;
            })
            .catch(function () {
                window.location.href = workspace;
            });
        return true;
    }

    function submitDashboardCommand(form, data) {
        return fetch(form.action, {
            method: "POST",
            body: data,
            redirect: "manual",
            headers: { "X-Requested-With": "fetch" }
        }).then(handleCommandResponse);
    }

    function redirectWorkspaceFromResponse(response) {
        var location = response.headers.get("Location") || response.headers.get("location");
        return location && appWorkspaceFromUrl(location);
    }

    function handleCommandRedirect(response) {
        if (response.status < 300) return null;
        if (response.status >= 400) return null;
        var workspace = redirectWorkspaceFromResponse(response);
        if (!workspace) return null;
        return loadWorkspace(workspace, true).then(function (loaded) {
            return loaded !== false;
        });
    }

    function handleCommandError(response) {
        return response.text().then(function (text) {
            setCommandStatus(text || "Dashboard command failed", "error");
            return false;
        });
    }

    function handleCommandResponse(response) {
        var redirect = handleCommandRedirect(response);
        if (redirect) return redirect;
        return response.ok ? true : handleCommandError(response);
    }

    function handleCommandFormSubmit(event, form) {
        event.preventDefault();
        var data = new FormData(form);
        var submittedCommand = String(data.get("command") || "");
        submitDashboardCommand(form, data).then(function (handled) {
            if (handled === false) return;
            pushCommandHistory(submittedCommand);
            setCommandStatus("", "");
            applyCommandSideEffect(submittedCommand);
            var input = form.querySelector("input[name='command']");
            if (input) input.value = "";
        }).catch(function () {
            form.submit();
        });
        return true;
    }

    document.addEventListener("submit", function (event) {
        var form = event.target;
        if (!form) return;
        if (!form.matches(".jaw-command form")) {
            handleWorkspaceFormSubmit(event, form);
            return;
        }
        handleCommandFormSubmit(event, form);
    });

    restorePanes();
})();
