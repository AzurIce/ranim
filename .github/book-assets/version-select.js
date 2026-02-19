(function () {
    // Resolve the base URL of the book site (e.g. /ranim-book/)
    var siteBase = (function () {
        var base = document.querySelector("base");
        if (base) return base.getAttribute("href").replace(/\/$/, "");
        // Fallback: strip known sub-paths from current path
        var m = location.pathname.match(/^(\/ranim-book)/);
        return m ? m[1] : "";
    })();

    var versionsUrl = siteBase + "/versions.json";

    // Detect current version from the URL path
    var currentVersion = (function () {
        var rel = location.pathname.slice(siteBase.length).replace(/^\//, "");
        var seg = rel.split("/")[0];
        // If the first segment looks like a version tag, we're in that version
        if (seg && /^v\d/.test(seg)) return seg;
        return "main";
    })();

    function createSelect(versions) {
        var select = document.createElement("select");
        select.className = "version-select";
        select.setAttribute("aria-label", "Select version");

        versions.forEach(function (v) {
            var opt = document.createElement("option");
            opt.value = v;
            opt.textContent = v;
            if (v === currentVersion) opt.selected = true;
            select.appendChild(opt);
        });

        select.addEventListener("change", function () {
            var target = select.value;
            var path = target === "main" ? siteBase + "/" : siteBase + "/" + target + "/";
            location.href = path;
        });

        return select;
    }

    function inject(versions) {
        var bar = document.getElementById("mdbook-menu-bar");
        if (!bar) return;

        var rightButtons = bar.querySelector(".right-buttons");
        if (!rightButtons) return;

        var select = createSelect(versions);
        rightButtons.insertBefore(select, rightButtons.firstChild);
    }

    // Fetch versions.json and inject the selector
    var xhr = new XMLHttpRequest();
    xhr.open("GET", versionsUrl);
    xhr.onload = function () {
        if (xhr.status === 200) {
            try {
                var versions = JSON.parse(xhr.responseText);
                if (Array.isArray(versions) && versions.length > 1) {
                    inject(versions);
                }
            } catch (e) { /* ignore parse errors */ }
        }
    };
    xhr.send();
})();
