(function () {
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
        if (push) {
            window.history.pushState(
                { workspace: next.getAttribute("data-workspace-url") || workspace },
                "",
                dashboardUrl(next.getAttribute("data-workspace-url") || workspace).toString()
            );
        }
        return true;
    }

    function setCommandStatus(message, kind) {
        var node = document.querySelector("[data-dashboard-command-status]");
        if (!node) return;
        node.textContent = message || "";
        node.dataset.statusKind = kind || "";
    }

    function paneStorageKey(pane) {
        return "icelines.dashboard.pane." + pane;
    }

    function setPaneVisible(pane, visible) {
        var node = document.querySelector("[data-dashboard-pane='" + pane + "']");
        if (!node) return;
        node.hidden = !visible;
        try {
            window.localStorage.setItem(paneStorageKey(pane), visible ? "show" : "hide");
        } catch (_) {}
    }

    function restorePanes() {
        ["favorites", "schedule"].forEach(function (pane) {
            try {
                if (window.localStorage.getItem(paneStorageKey(pane)) === "hide") {
                    setPaneVisible(pane, false);
                }
            } catch (_) {}
        });
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
            if (node) setPaneVisible(pane, node.hidden);
            return;
        }

        var link = event.target.closest("a[href]");
        if (!link) return;

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

    document.addEventListener("submit", function (event) {
        var form = event.target;
        if (!form || !form.matches(".jaw-command form")) return;

        event.preventDefault();
        var data = new FormData(form);
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
            setCommandStatus("", "");
            applyCommandSideEffect(String(data.get("command") || ""));
            var input = form.querySelector("input[name='command']");
            if (input) input.value = "";
        }).catch(function () {
            form.submit();
        });
    });

    restorePanes();
})();
