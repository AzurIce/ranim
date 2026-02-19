(function () {
    var siteBase = (function () {
        var base = document.querySelector("base");
        if (base) return base.getAttribute("href").replace(/\/$/, "");
        var m = location.pathname.match(/^(\/ranim-book)/);
        return m ? m[1] : "";
    })();

    var versionsUrl = siteBase + "/versions.json";

    function detectVersion() {
        var rel = location.pathname.slice(siteBase.length).replace(/^\//, "");
        var seg = rel.split("/")[0];
        if (seg && (seg === "main" || /^v\d/.test(seg))) return seg;
        return "main";
    }

    function createPicker(versions) {
        var current = detectVersion();

        var wrapper = document.createElement("div");
        wrapper.className = "version-picker";

        var btn = document.createElement("button");
        btn.className = "version-picker-btn";
        btn.setAttribute("type", "button");
        btn.setAttribute("aria-label", "Select version");
        btn.setAttribute("aria-haspopup", "true");
        btn.setAttribute("aria-expanded", "false");
        btn.textContent = current;

        var popup = document.createElement("ul");
        popup.className = "version-popup";

        versions.forEach(function (v) {
            var li = document.createElement("li");
            var a = document.createElement("a");
            a.href = siteBase + "/" + v + "/";
            a.textContent = v;
            if (v === current) a.className = "active";
            li.appendChild(a);
            popup.appendChild(li);
        });

        btn.addEventListener("click", function (e) {
            e.stopPropagation();
            var open = popup.classList.toggle("open");
            btn.setAttribute("aria-expanded", open);
        });

        document.addEventListener("click", function () {
            popup.classList.remove("open");
            btn.setAttribute("aria-expanded", "false");
        });

        wrapper.appendChild(btn);
        wrapper.appendChild(popup);
        return wrapper;
    }

    function inject(versions) {
        var bar = document.getElementById("mdbook-menu-bar") || document.getElementById("menu-bar");
        if (!bar) return;
        var rightButtons = bar.querySelector(".right-buttons");
        if (!rightButtons) return;
        rightButtons.insertBefore(createPicker(versions), rightButtons.firstChild);
    }

    function syncPicker() {
        var current = detectVersion();
        var btn = document.querySelector(".version-picker-btn");
        if (btn) btn.textContent = current;
        var links = document.querySelectorAll(".version-popup li a");
        for (var i = 0; i < links.length; i++) {
            if (links[i].textContent === current) {
                links[i].className = "active";
            } else {
                links[i].className = "";
            }
        }
    }

    var xhr = new XMLHttpRequest();
    xhr.open("GET", versionsUrl);
    xhr.onload = function () {
        if (xhr.status === 200) {
            try {
                var versions = JSON.parse(xhr.responseText);
                if (Array.isArray(versions) && versions.length > 1) {
                    inject(versions);
                }
            } catch (e) {}
        }
    };
    xhr.send();

    window.addEventListener("pageshow", function (e) {
        if (e.persisted) syncPicker();
    });
})();
