(function () {
    var commandHistory = readCommandHistory();
    var commandHistoryIndex = commandHistory.length;

    function dashboardWorkspaceFromUrl(url) {
        try {
            var parsed = new URL(url, window.location.origin);
            if (parsed.pathname !== "/dashboard") return null;
            return parsed.searchParams.get("workspace");
        } catch (_) {
            return null;
        }
    }

    function appWorkspaceFromUrl(url) {
        try {
            var parsed = new URL(url, window.location.origin);
            if (parsed.origin !== window.location.origin) return null;
            if (parsed.pathname === "/dashboard") return dashboardWorkspaceFromUrl(url);
            if (parsed.pathname.indexOf("/api/") === 0 || parsed.pathname.indexOf("/static/") === 0) return null;
            if (parsed.pathname.indexOf("/season-type/") === 0 || parsed.pathname === "/seasons") return null;
            var workspace = parsed.pathname + parsed.search;
            return isDashboardWorkspace(workspace) ? workspace : null;
        } catch (_) {
            return null;
        }
    }

    function isDashboardWorkspace(workspace) {
        var path = String(workspace || "").split("?")[0].replace(/\/+$/, "") || "/";
        if (/^\/player\/[0-9]+$/.test(path)) return true;
        if (/^\/team\/[A-Za-z]{2,3}(\/season)?$/.test(path)) return true;
        if (/^\/game\/[^/]+$/.test(path)) return true;
        return [
            "/", "/leaders", "/goalies", "/depth", "/poach", "/fantasy",
            "/scores", "/schedule", "/transactions", "/playoffs",
            "/favorites", "/watchlist", "/career", "/reports/poach",
            "/reports/weekly", "/admin", "/docs"
        ].indexOf(path) !== -1;
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
        if (!event.ctrlKey || event.metaKey || event.altKey) return null;
        return event.shiftKey ? "right" : "left";
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
        if (link.hasAttribute("data-dashboard-composition-link")) {
            if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
            if (!compositionUrl(link.href)) return;
            event.preventDefault();
            followDashboardComposition(link.href);
            return;
        }
        if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.altKey) return;
        var explicitTarget = link.getAttribute("data-dashboard-target");
        var paneTarget = explicitTarget || paneTargetFromClick(event);
        if (!paneTarget && event.shiftKey) return;

        var workspace = link.getAttribute("data-dashboard-workspace") || appWorkspaceFromUrl(link.href);
        if (!workspace) return;

        event.preventDefault();
        if (paneTarget === "left" || paneTarget === "right") {
            window.location.href = paneTargetUrl(workspace, paneTarget).toString();
            return;
        }
        loadWorkspace(workspace, true)
            .then(function (loaded) {
                if (!loaded) window.location.href = link.href;
            })
            .catch(function () {
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
        if (!isEditableTarget(event.target) && event.key === "]") {
            var rightRing = document.querySelector("[data-dashboard-ring='right']");
            if (rightRing) {
                event.preventDefault();
                followDashboardComposition(rightRing.href);
            }
            return;
        }
        if (!isEditableTarget(event.target) && event.key === "[") {
            var leftRing = document.querySelector("[data-dashboard-ring='left']");
            if (leftRing) {
                event.preventDefault();
                followDashboardComposition(leftRing.href);
            }
            return;
        }
        if (!isEditableTarget(event.target) && event.key === "\\") {
            var swap = document.querySelector("[data-dashboard-pane-swap='right'], [data-dashboard-pane-swap='left']");
            if (swap) {
                event.preventDefault();
                followDashboardComposition(swap.href);
            }
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
        if (!form) return;
        if (!form.matches(".jaw-command form")) {
            if (!form.closest(".jaw-workspace") || String(form.method || "get").toLowerCase() !== "get") return;

            event.preventDefault();
            var formUrl = new URL(form.action || window.location.href, window.location.origin);
            var params = new URLSearchParams(new FormData(form));
            var query = params.toString();
            var workspace = formUrl.pathname + (query ? "?" + query : "");
            if (!isDashboardWorkspace(workspace)) {
                window.location.href = workspace;
                return;
            }
            loadWorkspace(workspace, true)
                .then(function (loaded) {
                    if (!loaded) window.location.href = workspace;
                })
                .catch(function () {
                    window.location.href = workspace;
                });
            return;
        }

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
                var workspace = location && appWorkspaceFromUrl(location);
                if (workspace) {
                    return loadWorkspace(workspace, true).then(function (loaded) {
                        return loaded !== false;
                    });
                }
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
