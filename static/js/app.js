/* Interactions that Framer Motion and the React client used to own. */
(function () {
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  const HEADER_TOP =
    "mx-auto flex items-center justify-between gap-3 transition-all duration-500 max-w-6xl border border-transparent px-2 py-3";
  const HEADER_FLOAT =
    "mx-auto flex items-center justify-between gap-3 transition-all duration-500 max-w-[930px] rounded-full border border-navy/10 bg-white/70 px-4 py-2 shadow-[0_10px_40px_-12px_rgba(42,138,160,0.55)] backdrop-blur-xl";

  function initHeader() {
    const bar = document.getElementById("nav-bar");
    if (!bar) return;
    const on = () => {
      bar.className = window.scrollY > 40 ? HEADER_FLOAT : HEADER_TOP;
    };
    on();
    window.addEventListener("scroll", on, { passive: true });
  }

  function initCursor() {
    const el = document.getElementById("cursor");
    if (!el) return;
    const ok = window.matchMedia("(pointer: fine) and (prefers-reduced-motion: no-preference)").matches;
    if (!ok) return;
    let tx = -100;
    let ty = -100;
    let x = -100;
    let y = -100;
    let vx = 0;
    let vy = 0;
    window.addEventListener("pointermove", (e) => {
      tx = e.clientX;
      ty = e.clientY;
    });
    const step = () => {
      const stiffness = 400;
      const damping = 30;
      const dt = 0.016;
      vx += (-stiffness * (x - tx) - damping * vx) * dt;
      vy += (-stiffness * (y - ty) - damping * vy) * dt;
      x += vx * dt;
      y += vy * dt;
      el.style.transform = `translate(${x}px, ${y}px)`;
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }

  function cubicBezier(x1, y1, x2, y2) {
    return function (x) {
      if (x <= 0) return 0;
      if (x >= 1) return 1;
      let t = x;
      for (let i = 0; i < 8; i++) {
        const mt = 1 - t;
        const f = 3 * mt * mt * t * x1 + 3 * mt * t * t * x2 + t * t * t - x;
        const df = 3 * mt * mt * x1 + 6 * mt * t * (x2 - x1) + 3 * t * t * (1 - x2);
        if (Math.abs(df) < 1e-6) break;
        t = Math.min(1, Math.max(0, t - f / df));
      }
      const mt = 1 - t;
      return 3 * mt * mt * t * y1 + 3 * mt * t * t * y2 + t * t * t;
    };
  }

  function initCountUp() {
    const el = document.getElementById("soul-count");
    if (!el) return;
    const to = Number(el.dataset.to);
    const width = String(to).length;
    const paint = (v) => {
      el.textContent = String(v).padStart(width, "0");
    };
    if (reduce || !Number.isFinite(to)) {
      paint(to);
      return;
    }
    const ease = cubicBezier(0.22, 1, 0.36, 1);
    const start = performance.now();
    const dur = 2200;
    const tick = (now) => {
      const t = Math.min(1, (now - start) / dur);
      paint(Math.round(ease(t) * to));
      if (t < 1) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }

  function initReveals() {
    if (reduce) return;
    const watch = (selector, margin) => {
      const io = new IntersectionObserver(
        (entries) => {
          for (const e of entries) {
            if (!e.isIntersecting) continue;
            e.target.classList.add("in");
            io.unobserve(e.target);
          }
        },
        { rootMargin: margin },
      );
      document.querySelectorAll(selector).forEach((el) => io.observe(el));
    };
    watch(".reveal", "-60px");
    watch(".word", "-40px");
  }

  function initApplicants() {
    const slot = document.getElementById("applicant-count");
    if (!slot) return;
    fetch("/api/stats")
      .then((r) => r.json())
      .then((d) => {
        if (typeof d.applicants !== "number") return;
        slot.classList.remove("hidden");
        slot.innerHTML = `<span class="font-bold text-peach-deep">${d.applicants.toLocaleString("en-US")}</span> wanderers have applied`;
      })
      .catch(() => {});
  }

  function wrapIndex(i, d, n) {
    return (((i + d) % n) + n) % n;
  }

  function initDeck() {
    const card = document.getElementById("legend-card");
    const dataEl = document.getElementById("legends-data");
    if (!card || !dataEl) return;
    let legends = [];
    try {
      legends = JSON.parse(dataEl.textContent || "[]");
    } catch {
      return;
    }
    const n = legends.length;
    if (!n) return;
    let index = 0;
    const counter = document.getElementById("legend-index");
    const fill = (i) => {
      const cur = legends[i];
      const badge = String(i + 1).padStart(3, "0");
      card.style.boxShadow = `0 24px 60px -18px ${cur.color}, 0 0 0 6px ${cur.color}33`;
      card.innerHTML = `<img src="${cur.image}" alt="Lethean #${badge} — sample" data-nft="1" data-color="${cur.color}" draggable="false" class="pointer-events-none aspect-square w-full rounded-[28px] object-cover"><div class="flex flex-1 items-center justify-between px-2"><span class="label rounded-full border border-peach-deep/40 bg-peach-pale/50 px-3 py-1 text-peach-deep" aria-live="polite">#${badge}</span><span class="label text-navy/50">drag or flick</span></div>`;
      if (counter) counter.textContent = String(i + 1).padStart(2, "0");
    };
    const go = (dir) => {
      const next = wrapIndex(index, dir, n);
      const apply = () => {
        index = next;
        fill(index);
        card.style.transform = "";
        if (!reduce) {
          card.animate(
            [
              { opacity: 0, transform: "translateY(30px) scale(0.9)" },
              { opacity: 1, transform: "none" },
            ],
            { duration: 450, easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
          );
        }
      };
      if (reduce) {
        apply();
        return;
      }
      card.style.transform = "";
      const anim = card.animate(
        [
          { opacity: 1, transform: "none" },
          { opacity: 0, transform: `translateX(${dir * -300}px) rotate(${dir * -14}deg)` },
        ],
        { duration: 350, easing: "ease" },
      );
      anim.onfinish = apply;
    };
    document.getElementById("legend-prev")?.addEventListener("click", () => go(-1));
    document.getElementById("legend-next")?.addEventListener("click", () => go(1));
    if (!reduce) {
      let dragging = false;
      let startX = 0;
      let lastX = 0;
      let lastT = 0;
      let vx = 0;
      card.addEventListener("pointerdown", (e) => {
        dragging = true;
        startX = lastX = e.clientX;
        lastT = e.timeStamp;
        vx = 0;
        card.setPointerCapture?.(e.pointerId);
      });
      card.addEventListener("pointermove", (e) => {
        if (!dragging) return;
        const dx = e.clientX - startX;
        const dt = e.timeStamp - lastT || 16;
        vx = ((e.clientX - lastX) / dt) * 1000;
        lastX = e.clientX;
        lastT = e.timeStamp;
        card.style.transform = `translateX(${dx}px) rotate(${dx / 25}deg)`;
      });
      const end = (e) => {
        if (!dragging) return;
        dragging = false;
        const dx = e.clientX - startX;
        card.style.transform = "";
        if (Math.abs(dx) > 80 || Math.abs(vx) > 500) go(dx < 0 ? 1 : -1);
      };
      card.addEventListener("pointerup", end);
      card.addEventListener("pointercancel", () => {
        dragging = false;
        card.style.transform = "";
      });
    }
  }

  function initCountdown() {
    document.querySelectorAll("[data-countdown]").forEach((root) => {
      const t = new Date(root.dataset.countdown).getTime();
      if (Number.isNaN(t)) return;
      const tick = () => {
        const now = Date.now();
        if (now >= t) {
          const p = document.createElement("p");
          p.className = "font-display text-3xl font-semibold text-peach-deep";
          p.textContent = "The river is open — mint is live!";
          root.replaceWith(p);
          clearInterval(id);
          return;
        }
        const s = Math.max(0, Math.floor((t - now) / 1000));
        const parts = {
          Days: Math.floor(s / 86400),
          Hours: Math.floor((s % 86400) / 3600),
          Mins: Math.floor((s % 3600) / 60),
          Secs: s % 60,
        };
        for (const [k, v] of Object.entries(parts)) {
          const cell = root.querySelector(`[data-unit="${k}"]`);
          if (cell) cell.textContent = String(v).padStart(2, "0");
        }
      };
      tick();
      const id = setInterval(tick, 1000);
    });
  }

  const KNOT = `<svg viewBox="0 0 48 48" class="h-1/3 w-1/3" aria-hidden="true"><g transform="rotate(45 24 24)" fill="none" stroke="#fff" stroke-width="2.6" stroke-linejoin="round"><path d="M14 14h20v20H14zM14 20.7h20M14 27.3h20M20.7 14v20M27.3 14v20"></path></g></svg>`;

  function initNftFallback() {
    document.addEventListener(
      "error",
      (e) => {
        const img = e.target;
        if (!(img instanceof HTMLImageElement) || !img.dataset.nft) return;
        const color = img.dataset.color || "#79d3df";
        const div = document.createElement("div");
        div.className = `flex items-center justify-center ${img.className}`;
        div.style.cssText = img.style.cssText;
        div.style.background = `linear-gradient(135deg, ${color}, #0e3039)`;
        div.setAttribute("role", "img");
        div.setAttribute("aria-label", img.alt || "");
        div.innerHTML = KNOT;
        img.replaceWith(div);
      },
      true,
    );
  }

  function parseXStatusUrl(raw) {
    if (!raw) return null;
    let u;
    try {
      u = new URL(String(raw).trim());
    } catch {
      return null;
    }
    if (u.protocol !== "https:" && u.protocol !== "http:") return null;
    if (!/^(www\.|mobile\.)?(x|twitter)\.com$/i.test(u.hostname)) return null;
    const m = u.pathname.match(/^\/(?:i\/web|i|[A-Za-z0-9_]{1,15})\/status\/(\d{5,25})\/?/);
    if (!m) return null;
    const first = u.pathname.split("/")[1];
    return { user: first.toLowerCase(), id: m[1] };
  }

  const isAddr = (v) => /^0x[0-9a-fA-F]{40}$/.test(v.trim());

  function initApply() {
    const root = document.getElementById("apply-root");
    const form = document.getElementById("apply-form");
    if (!root || !form) return;
    const signedIn = root.dataset.signedIn === "1";
    const demo = root.dataset.demo === "1";
    const username = root.dataset.username || "";
    const state = { opened: {}, completed: {} };

    const tasks = [...form.querySelectorAll("[data-task-id]")].map((li) => ({
      li,
      id: li.dataset.taskId,
      required: li.dataset.required === "1",
      requiresLink: li.dataset.requiresLink === "1",
    }));

    const isDone = (t) => {
      if (t.requiresLink) {
        const input = t.li.querySelector("[data-link]");
        return Boolean(state.completed[t.id]) && Boolean(parseXStatusUrl(input?.value));
      }
      return Boolean(state.opened[t.id]);
    };

    const render = () => {
      const progress = tasks.filter(isDone).length;
      const prog = document.getElementById("quest-progress");
      if (prog) prog.textContent = String(progress);
      const requiredDone = tasks.filter((t) => t.required).every(isDone);
      const wallet = document.getElementById("wallet");
      const walletOk = wallet ? isAddr(wallet.value) : false;
      for (const t of tasks) {
        const done = isDone(t);
        t.li.className = `rounded-2xl border p-4 transition-colors duration-500 ${done ? "border-mint/60 bg-mint/15" : "border-sky/15 bg-white/[0.03]"}`;
        const flag = t.li.querySelector("[data-done-flag]");
        if (flag) {
          flag.classList.toggle("hidden", !done);
          flag.classList.toggle("flex", done);
        }
        const input = t.li.querySelector("[data-link]");
        if (input) {
          input.readOnly = done;
          const valid = parseXStatusUrl(input.value);
          const bad = Boolean(input.value) && !valid;
          input.setAttribute("aria-invalid", bad ? "true" : "false");
          const help = t.li.querySelector("[data-link-help]");
          if (help) {
            help.textContent = bad
              ? "That doesn't look like an X post link (x.com/yourname/status/…)."
              : "On X, tap Share on your reply, then Copy link, and paste it here.";
            help.className = `mt-1 text-xs ${bad ? "text-peach-deep" : "text-current/55"}`;
          }
          const complete = t.li.querySelector("[data-complete]");
          if (complete) {
            complete.classList.toggle("hidden", done);
            complete.disabled = !signedIn || !valid;
          }
        }
      }
      const walletStep = form.querySelector("[data-wallet-step]");
      if (walletStep) {
        const open = signedIn && requiredDone;
        walletStep.disabled = !open;
        walletStep.classList.toggle("opacity-60", !open);
      }
      const help = document.getElementById("wallet-help");
      if (help && wallet) {
        const bad = wallet.value.length > 0 && !walletOk;
        wallet.setAttribute("aria-invalid", bad ? "true" : "false");
        help.textContent = bad
          ? "That isn't a valid 0x address (42 characters)."
          : "Use a wallet you control. No exchange addresses.";
        help.className = `mt-1 text-xs ${bad ? "text-peach-deep" : "text-current/55"}`;
      }
      const step = !signedIn ? 0 : !requiredDone ? 1 : !walletOk ? 2 : 3;
      form.querySelectorAll("[data-step]").forEach((li, i) => {
        const span = li.querySelector("span");
        if (i === step) li.setAttribute("aria-current", "step");
        else li.removeAttribute("aria-current");
        if (!span) return;
        span.textContent = i < step ? "✓" : String(i + 1);
        span.className = `mx-auto flex h-8 w-8 items-center justify-center rounded-full border text-sm font-semibold transition ${
          i < step
            ? "border-mint bg-mint text-white"
            : i === step
              ? "border-peach text-peach-deep shadow-[0_0_16px_rgba(239,122,69,0.45)]"
              : "border-sky/25 text-current/40"
        }`;
      });
      const submit = document.getElementById("apply-submit");
      const can = signedIn && requiredDone && walletOk && state.status !== "submitting" && !demo;
      if (submit) {
        submit.disabled = !can;
        submit.textContent = state.status === "submitting" ? "Sending to the river…" : "Submit application";
      }
    };

    state.status = "idle";
    form.querySelectorAll("[data-open]").forEach((a) => {
      a.addEventListener("click", (e) => {
        if (!signedIn) {
          e.preventDefault();
          return;
        }
        const t = tasks.find((x) => x.li.contains(a));
        if (t && !t.requiresLink) state.opened[t.id] = true;
        render();
      });
    });
    form.querySelectorAll("[data-link]").forEach((input) => input.addEventListener("input", render));
    form.querySelectorAll("[data-complete]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const t = tasks.find((x) => x.li.contains(btn));
        if (!t) return;
        const input = t.li.querySelector("[data-link]");
        if (signedIn && parseXStatusUrl(input?.value)) {
          state.completed[t.id] = true;
          render();
        }
      });
    });
    document.getElementById("wallet")?.addEventListener("input", render);

    const showError = (msg) => {
      state.status = "error";
      const p = document.getElementById("apply-error");
      if (p) {
        p.textContent = msg;
        p.classList.remove("hidden");
      }
      render();
    };

    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const wallet = document.getElementById("wallet")?.value || "";
      const requiredDone = tasks.filter((t) => t.required).every(isDone);
      if (!(signedIn && requiredDone && isAddr(wallet) && state.status !== "submitting" && !demo)) return;
      state.status = "submitting";
      const err = document.getElementById("apply-error");
      err?.classList.add("hidden");
      render();
      const linkTask = tasks.find((t) => t.requiresLink);
      try {
        const res = await fetch("/api/apply", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            wallet,
            website: document.getElementById("website")?.value || "",
            tasks: tasks.filter(isDone).map((t) => t.id),
            replyUrl: linkTask?.li.querySelector("[data-link]")?.value || "",
          }),
        });
        const data = await res.json().catch(() => ({}));
        if (res.ok && data.ok) {
          const mine = (await fetchMine()) || {
            number: typeof data.number === "number" ? data.number : null,
            username: username || null,
            wallet: `${wallet.slice(0, 6)}…${wallet.slice(-4)}`,
            status: "pending",
          };
          showDone({ fresh: true, username, mine });
        } else if (data.code === "already_applied") {
          const mine = await fetchMine();
          if (mine) showDone({ fresh: false, username, mine });
          else showDone({ fresh: false, username, mine: null, msg: data.message });
        } else {
          showError(data.message || "Something went wrong. Please try again.");
        }
      } catch {
        showError("Network error. Check your connection and try again.");
      }
    });

    render();

    if (signedIn && !demo) {
      fetchMine().then((mine) => {
        document.getElementById("apply-checking")?.remove();
        if (mine) showDone({ fresh: false, username, mine });
        else form.classList.remove("hidden");
      });
    }
  }

  async function fetchMine() {
    try {
      const r = await fetch("/api/wl-status?me=1", { cache: "no-store" });
      const j = await r.json().catch(() => ({}));
      if (r.ok && j.ok && j.found) {
        return { number: j.number, username: j.username, wallet: j.wallet, status: j.status };
      }
    } catch {
      /* ignore */
    }
    return null;
  }

  function showDone({ fresh, username, mine, msg }) {
    const tpl = document.getElementById("tpl-apply-done");
    const root = document.getElementById("apply-root");
    if (!tpl || !root) return;
    root.replaceChildren(tpl.content.cloneNode(true));
    const emoji = root.querySelector("[data-emoji]");
    const title = root.querySelector("[data-title]");
    const body = root.querySelector("[data-body]");
    if (emoji) emoji.textContent = fresh ? "🏮" : "🫧";
    if (title) title.textContent = fresh ? "You're at the river's edge!" : "You've already applied";
    const who = username || mine?.username || "";
    if (body) {
      body.textContent = fresh
        ? `Application received for @${username}. Stay tuned to our X for updates.`
        : `@${who} is already on the list. Each X account can apply once.`;
    }
    if (mine) mountCard(root.querySelector("[data-card]"), mine);
    else if (msg) {
      const p = root.querySelector("[data-msg]");
      if (p) {
        p.textContent = msg;
        p.classList.remove("hidden");
      }
    }
  }

  function cardAria(data) {
    const num = data.number ? ` number ${data.number}` : "";
    const user = data.username ? ` for @${data.username}` : "";
    const status = data.status === "pending" ? "pending review" : data.status;
    return `Letheans whitelist card${num}${user}, status ${status}`;
  }

  function mountCard(host, data) {
    if (!host || !window.LetheansCard) return;
    const tpl = document.getElementById("tpl-wl-card");
    if (!tpl) return;
    host.replaceChildren(tpl.content.cloneNode(true));
    const canvas = host.querySelector("canvas");
    const shareHost = document.body.dataset.shareHost || "letheans.xyz";
    const full = { ...data, host: shareHost };
    canvas.setAttribute("role", "img");
    canvas.setAttribute("aria-label", cardAria(data));
    void window.LetheansCard.drawCard(canvas, full);
    const ta = host.querySelector("textarea");
    const share = host.querySelector("[data-act='share']");
    const setCaption = (value) => {
      if (ta) ta.value = value;
      if (share) share.href = `https://x.com/intent/post?text=${encodeURIComponent(value)}`;
    };
    setCaption(window.LetheansCard.cardCaption(full));
    ta?.addEventListener("input", () => setCaption(ta.value));
    const fileName = `letheans-${data.number ? `soul-${String(data.number).padStart(4, "0")}` : "wl-card"}.png`;
    const toBlob = () =>
      new Promise((resolve) => (canvas ? canvas.toBlob(resolve, "image/png") : resolve(null)));
    host.querySelector("[data-act='download']")?.addEventListener("click", async () => {
      const blob = await toBlob();
      if (!blob) return;
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = fileName;
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 2000);
    });
    const flash = (btn, label) => {
      if (!btn) return;
      const prev = btn.textContent;
      btn.classList.add("btn-success");
      btn.textContent = "Copied ✓";
      setTimeout(() => {
        btn.classList.remove("btn-success");
        btn.textContent = prev === "Copied ✓" ? label : prev;
      }, 2000);
    };
    host.querySelector("[data-act='copy-image']")?.addEventListener("click", async (e) => {
      const btn = e.currentTarget;
      try {
        await navigator.clipboard.write([new ClipboardItem({ "image/png": toBlob().then((b) => b ?? Promise.reject()) })]);
        flash(btn, "Copy image");
      } catch {
        /* clipboard image not supported: download still works */
      }
    });
    host.querySelector("[data-act='copy-caption']")?.addEventListener("click", async (e) => {
      const btn = e.currentTarget;
      try {
        await navigator.clipboard.writeText(ta?.value || "");
        flash(btn, "Copy caption");
      } catch {
        /* ignore */
      }
    });
  }

  function initStatus() {
    const form = document.getElementById("status-form");
    const input = document.getElementById("wl-q");
    const button = document.getElementById("status-submit");
    const result = document.getElementById("status-result");
    if (!form || !input || !button || !result) return;
    let loading = false;
    const sync = () => {
      button.disabled = !input.value.trim() || loading;
      button.textContent = loading ? "Checking…" : "Check WL Status";
    };
    input.addEventListener("input", sync);
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const value = input.value.trim();
      if (!value || loading) return;
      loading = true;
      sync();
      try {
        const r = await fetch(`/api/wl-status?q=${encodeURIComponent(value)}`, { cache: "no-store" });
        const j = await r.json().catch(() => ({}));
        if (!r.ok || !j.ok) {
          const tpl = document.getElementById("tpl-status-error");
          result.replaceChildren(tpl.content.cloneNode(true));
          result.querySelector("[data-msg]").textContent = j.message || "Something went wrong. Please try again.";
        } else if (!j.found) {
          result.replaceChildren(document.getElementById("tpl-status-none").content.cloneNode(true));
        } else {
          result.replaceChildren();
          mountCard(result, { number: j.number, username: j.username, wallet: j.wallet, status: j.status });
        }
      } catch {
        const tpl = document.getElementById("tpl-status-error");
        result.replaceChildren(tpl.content.cloneNode(true));
        result.querySelector("[data-msg]").textContent = "Network error. Check your connection and try again.";
      } finally {
        loading = false;
        sync();
      }
    });
  }

  initHeader();
  initCursor();
  initCountUp();
  initReveals();
  initApplicants();
  initDeck();
  initCountdown();
  initNftFallback();
  initApply();
  initStatus();
})();
