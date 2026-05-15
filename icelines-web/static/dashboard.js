(function () {
    var commandHistory = readCommandHistory();
    var commandHistoryIndex = commandHistory.length;

    function workspaceFromUrl(url) {
        try {
            var parsed = new URL(url, window.location.origin);
            if (parsed.pathname !== "/dashboard") return null;
            return parsed.searchParams.get("workspace");
        } catch (_) {
            return null;
        }
    }

    function dashboardUrl(workspace) {
        var url = new URL("/dashboard", window.location.origin);
        url.searchParams.set("workspace", workspace || "/leaders");
        return url;
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

        var html = await response.text();
        var template = document.createElement("template");
        template.innerHTML = html.trim();
        var next = template.content.querySelector(".jaw-workspace");
        if (!next) return false;

        current.replaceWith(next);
        var nextWorkspace = next.getAttribute("data-workspace-url") || workspace;
        updateCommandWorkspace(nextWorkspace);
        if (push) {
            window.history.pushState(
                { workspace: nextWorkspace },
                "",
                dashboardUrl(nextWorkspace).toString()
            );
        }
        return true;
    }

    function setCommandStatus(message, kind) {
        var node = document.querySelector("[data-dashboard-command-status]");
        if (!node) return;
        var text = message || "";
        if (kind === "error" && text && !/^error[:\s]/i.test(text)) {
            text = "Error: " + text;
        }
        node.textContent = text;
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
        return tag === "input" || tag === "textarea" || tag === "select" || target.isContentEditable;
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
        commandHistoryIndex += direction;
        if (commandHistoryIndex < 0) commandHistoryIndex = 0;
        if (commandHistoryIndex > commandHistory.length) commandHistoryIndex = commandHistory.length;
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

    function setPaneVisible(pane, visible, persist) {
        var node = document.querySelector("[data-dashboard-pane='" + pane + "']");
        var toggle = document.querySelector("[data-dashboard-pane-toggle='" + pane + "']");
        if (node) node.setAttribute("data-dashboard-pane-collapsed", visible ? "false" : "true");
        if (toggle) {
            toggle.setAttribute("aria-expanded", visible ? "true" : "false");
            toggle.textContent = visible ? "Hide" : "Show";
        }
        if (!node) return;
        if (persist === false) return;
        try {
            window.localStorage.setItem(paneStorageKey(pane), visible ? "show" : "hide");
        } catch (_) {}
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
        if (normalized === "/hide favorites" || normalized === "/hide fav") {
            setPaneVisible("favorites", false);
        } else if (normalized === "/show favorites" || normalized === "/show fav") {
            setPaneVisible("favorites", true);
        } else if (normalized === "/hide schedule" || normalized === "/hide sched") {
            setPaneVisible("schedule", false);
        } else if (normalized === "/show schedule" || normalized === "/show sched") {
            setPaneVisible("schedule", true);
        }
    }

    document.addEventListener("click", function (event) {
        var toggle = event.target.closest("[data-dashboard-pane-toggle]");
        if (toggle) {
            event.preventDefault();
            var pane = toggle.getAttribute("data-dashboard-pane-toggle");
            var node = document.querySelector("[data-dashboard-pane='" + pane + "']");
            if (node) setPaneVisible(pane, node.getAttribute("data-dashboard-pane-collapsed") === "true");
            return;
        }

        var link = event.target.closest("a[href]");
        if (!link) return;
        if (link.hasAttribute("data-dashboard-composition-link")) return;

        var workspace = workspaceFromUrl(link.href);
        if (!workspace) return;

        event.preventDefault();
        loadWorkspace(workspace, true).catch(function () {
            window.location.href = link.href;
        });
    });

    window.addEventListener("popstate", function () {
        var params = new URLSearchParams(window.location.search);
        var workspace = params.get("workspace") || "/leaders";
        loadWorkspace(workspace, false).catch(function () {
            window.location.href = dashboardUrl(workspace).toString();
        });
    });

    document.addEventListener("keydown", function (event) {
        if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
            event.preventDefault();
            focusCommandInput();
            return;
        }
        if (event.key === "/" && !isEditableTarget(event.target)) {
            event.preventDefault();
            focusCommandInput();
            return;
        }
        if (event.key === "Escape" && event.target === commandInput()) {
            event.preventDefault();
            event.target.value = "";
            setCommandStatus("", "");
            return;
        }
        if (event.target === commandInput() && event.key === "ArrowUp") {
            event.preventDefault();
            recallCommandHistory(event.target, -1);
            return;
        }
        if (event.target === commandInput() && event.key === "ArrowDown") {
            event.preventDefault();
            recallCommandHistory(event.target, 1);
        }
    });

    document.addEventListener("submit", function (event) {
        var form = event.target;
        if (!form || !form.matches(".jaw-command form")) return;

        event.preventDefault();
        var data = new FormData(form);
        var submittedCommand = String(data.get("command") || "");
        fetch(form.action, {
            method: "POST",
            body: data,
            redirect: "manual",
            headers: { "X-Requested-With": "fetch" }
        }).then(function (response) {
            if (response.status >= 300 && response.status < 400) {
                var location = response.headers.get("Location") || response.headers.get("location");
                var workspace = location && workspaceFromUrl(location);
                if (workspace) return loadWorkspace(workspace, true);
            }
            if (!response.ok) {
                return response.text().then(function (text) {
                    setCommandStatus(text || "Dashboard command failed", "error");
                    return false;
                });
            }
            return true;
        }).then(function (handled) {
            if (handled === false) return;
            pushCommandHistory(submittedCommand);
            setCommandStatus("", "");
            applyCommandSideEffect(submittedCommand);
            var input = form.querySelector("input[name='command']");
            if (input) input.value = "";
        }).catch(function () {
            form.submit();
        });
    });

    restorePanes();
})();
