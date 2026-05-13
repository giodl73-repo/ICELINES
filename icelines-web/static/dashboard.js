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

    document.addEventListener("click", function (event) {
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
                    throw new Error(text || "Dashboard command failed");
                });
            }
            return true;
        }).then(function () {
            var input = form.querySelector("input[name='command']");
            if (input) input.value = "";
        }).catch(function () {
            form.submit();
        });
    });
})();
