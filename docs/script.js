(() => {
  const REPO = "DevDad-Main/Visus";
  const RELEASES = `https://github.com/${REPO}/releases/latest`;

  const os = (() => {
    const ua = navigator.userAgent || "";
    if (/Windows/i.test(ua)) return "windows";
    if (/Linux|X11/i.test(ua) && !/Android/i.test(ua)) return "linux";
    return "other";
  })();

  // Live clock in the hero pill.
  const pad = (n) => String(n).padStart(2, "0");
  const tick = () => {
    const d = new Date();
    const el = document.getElementById("clock");
    if (el) el.textContent = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  };
  tick();
  setInterval(tick, 1000);

  // Page tour tabs.
  const img = document.getElementById("show-img");
  const tabs = [...document.querySelectorAll(".tabs button")];
  const show = (btn) => {
    tabs.forEach((b) => b.classList.toggle("on", b === btn));
    img.classList.add("swap");
    setTimeout(() => {
      img.src = `assets/shots/${btn.dataset.shot}.webp`;
      img.width = btn.dataset.w;
      img.height = btn.dataset.h;
      img.alt = `Visus ${btn.textContent.toLowerCase()} page`;
      img.onload = () => img.classList.remove("swap");
    }, 220);
  };
  tabs.forEach((b) => b.addEventListener("click", () => show(b)));
  let auto = setInterval(() => show(tabs[(tabs.findIndex((b) => b.classList.contains("on")) + 1) % tabs.length]), 4500);
  document.querySelector(".tabs")?.addEventListener("click", () => clearInterval(auto));

  // Scroll reveal.
  const targets = document.querySelectorAll(".split > *, .frame, .grid article, .looks figure, .download .wrap > *, .features .wrap > p, .features h2");
  targets.forEach((el, i) => {
    el.classList.add("rv");
    el.style.transitionDelay = `${(i % 3) * 90}ms`;
  });
  const io = new IntersectionObserver(
    (entries) => entries.forEach((e) => e.isIntersecting && (e.target.classList.add("in"), io.unobserve(e.target))),
    { threshold: 0.12 }
  );
  targets.forEach((el) => io.observe(el));

  // Resolve the newest release and pick the right file for this machine.
  const pick = (assets) => {
    const by = (re) => assets.find((a) => re.test(a.name));
    return {
      windows: by(/Setup\.exe$/i),
      windowsZip: by(/windows.*\.zip$/i),
      linux: by(/linux.*\.tar\.gz$/i),
    };
  };
  const size = (b) => `${(b / 1048576).toFixed(1)} MB`;

  fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { Accept: "application/vnd.github+json" } })
    .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
    .then((rel) => {
      const f = pick(rel.assets || []);
      const main = os === "linux" ? f.linux : f.windows;
      const href = main ? main.browser_download_url : RELEASES;
      const label = os === "linux" ? "Download for Linux" : os === "windows" ? "Download for Windows" : "Download Visus";
      document.querySelectorAll("[data-dl]").forEach((a) => (a.href = href));
      const big = document.querySelector("[data-dl-main]");
      if (big) big.href = href;
      document.querySelectorAll("[data-dl-label]").forEach((s) => (s.textContent = label));
      const ver = rel.tag_name || "";
      const date = rel.published_at ? new Date(rel.published_at).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }) : "";
      document.querySelectorAll("[data-dl-meta]").forEach((m) => (m.textContent = [ver, main && size(main.size), date].filter(Boolean).join("  ·  ")));
      const all = document.getElementById("all-dl");
      if (all) {
        const links = [
          [f.windows, "Windows installer"],
          [f.windowsZip, "Windows portable (.zip)"],
          [f.linux, "Linux (.tar.gz)"],
        ];
        all.innerHTML = "";
        links.forEach(([asset, name]) => {
          if (!asset) return;
          const a = document.createElement("a");
          a.href = asset.browser_download_url;
          a.textContent = `${name} · ${size(asset.size)}`;
          all.appendChild(a);
        });
      }
    })
    .catch(() => {
      document.querySelectorAll("[data-dl]").forEach((a) => (a.href = RELEASES));
    });
})();
