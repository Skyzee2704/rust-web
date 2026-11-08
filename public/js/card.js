/* Client-side 16:9 registration card. Port of src/lib/card.ts (1600x900 at 2x). */
(function () {
  const CARD_W = 1600;
  const CARD_H = 900;
  const CARD_SCALE = 2;

  const STATUS_LABEL = {
    pending: "PENDING REVIEW",
    GTD: "GTD",
    FCFS: "FCFS",
  };
  const STATUS_COLORS = {
    pending: { bg: "#fff0df", fg: "#0e3039", sub: "Your soul is being judged" },
    GTD: { bg: "#ffd978", fg: "#0e3039", sub: "Guaranteed spot · whitelisted" },
    FCFS: { bg: "#a8f0ea", fg: "#0e3039", sub: "First come, first served · whitelisted" },
  };

  function cardCaption(d) {
    const num = d.number ? `Soul #${String(d.number).padStart(4, "0")}` : "my soul";
    const url = `https://${d.host}`;
    if (d.status === "GTD")
      return `The river has judged. I'm on the GTD whitelist for @LetheansNFT 🌊\n${num} is guaranteed a drink.\n\nDrink. Forget. Reborn.\n${url} #Letheans`;
    if (d.status === "FCFS")
      return `The river has judged. I'm on the FCFS whitelist for @LetheansNFT 🌊\n${num} is ready to drink.\n\nDrink. Forget. Reborn.\n${url} #Letheans`;
    return `I just stepped up to the river and applied for the @LetheansNFT whitelist 🌊\n${num} is waiting to be judged.\n\nDrink. Forget. Reborn.\n${url} #Letheans`;
  }

  function loadImage(src) {
    return new Promise((resolve) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => resolve(null);
      img.src = src;
    });
  }

  function fontFamilies() {
    const cs = getComputedStyle(document.documentElement);
    const pick = (v, fb) => cs.getPropertyValue(v).trim() || fb;
    return {
      display: pick("--font-playfair", "Georgia, serif"),
      sans: pick("--font-inter", "system-ui, sans-serif"),
    };
  }

  function rr(ctx, x, y, w, h, r) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.arcTo(x + w, y, x + w, y + h, r);
    ctx.arcTo(x + w, y + h, x, y + h, r);
    ctx.arcTo(x, y + h, x, y, r);
    ctx.arcTo(x, y, x + w, y, r);
    ctx.closePath();
  }

  async function drawCard(canvas, d) {
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    canvas.width = CARD_W * CARD_SCALE;
    canvas.height = CARD_H * CARD_SCALE;
    ctx.setTransform(CARD_SCALE, 0, 0, CARD_SCALE, 0, 0);
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = "high";
    const f = fontFamilies();
    try {
      await Promise.all([
        document.fonts.load(`700 120px ${f.display}`),
        document.fonts.load(`600 40px ${f.sans}`),
        document.fonts.load(`500 30px ${f.sans}`),
      ]);
    } catch {
      /* fall back to the stack */
    }
    const [logo, avatar, badge] = await Promise.all([
      loadImage("/logo-arch.png"),
      d.username ? loadImage(`/api/avatar?u=${encodeURIComponent(d.username)}`) : Promise.resolve(null),
      loadImage("/logo.png"),
    ]);

    const g = ctx.createLinearGradient(0, 0, CARD_W * 0.35, CARD_H);
    g.addColorStop(0, "#2a8aa0");
    g.addColorStop(0.45, "#79ccd8");
    g.addColorStop(0.8, "#bfe8ee");
    g.addColorStop(1, "#fff0df");
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, CARD_W, CARD_H);

    ctx.save();
    ctx.strokeStyle = "rgba(255,255,255,0.16)";
    ctx.lineWidth = 6;
    for (let i = -CARD_H; i < CARD_W + CARD_H; i += 90) {
      ctx.beginPath();
      ctx.moveTo(i, 0);
      ctx.lineTo(i + CARD_H, CARD_H);
      ctx.moveTo(i + CARD_H, 0);
      ctx.lineTo(i, CARD_H);
      ctx.stroke();
    }
    ctx.restore();

    ctx.fillStyle = "#0e3039";
    ctx.fillRect(0, CARD_H - 120, CARD_W, 120);

    const ax = 1010;
    const aw = 500;
    const ah = 640;
    const ay = CARD_H - 120 - ah + 10;
    ctx.save();
    ctx.beginPath();
    ctx.moveTo(ax, ay + ah);
    ctx.lineTo(ax, ay + aw / 2);
    ctx.arc(ax + aw / 2, ay + aw / 2, aw / 2, Math.PI, 0);
    ctx.lineTo(ax + aw, ay + ah);
    ctx.closePath();
    ctx.fillStyle = "rgba(255,255,255,0.35)";
    ctx.fill();
    ctx.lineWidth = 10;
    ctx.strokeStyle = "rgba(255,255,255,0.9)";
    ctx.stroke();
    ctx.clip();
    const pic = avatar || logo;
    if (pic) {
      const scale = Math.max(aw / pic.width, ah / pic.height);
      const dw = pic.width * scale;
      const dh = pic.height * scale;
      ctx.drawImage(pic, ax + (aw - dw) / 2, ay + (ah - dh) / 2, dw, dh);
    }
    ctx.restore();
    ctx.fillStyle = "#0e3039";
    ctx.fillRect(0, CARD_H - 120, CARD_W, 120);

    if (badge) {
      const br = 44;
      const bx = 90 + br;
      const by = 108;
      ctx.save();
      ctx.beginPath();
      ctx.arc(bx, by, br, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(255,255,255,0.9)";
      ctx.fill();
      ctx.beginPath();
      ctx.arc(bx, by, br - 4, 0, Math.PI * 2);
      ctx.clip();
      ctx.drawImage(badge, bx - br, by - br, br * 2, br * 2);
      ctx.restore();
    }

    ctx.fillStyle = "#0e3039";
    ctx.font = `700 54px ${f.display}`;
    ctx.textBaseline = "alphabetic";
    ctx.fillText("LETHEANS", 204, 120);
    ctx.font = `500 22px ${f.sans}`;
    ctx.fillStyle = "rgba(14,48,57,0.7)";
    ctx.fillText("WHITELIST CARD  ·  ELYSIUM", 208, 158);

    ctx.fillStyle = "rgba(14,48,57,0.65)";
    ctx.font = `600 26px ${f.sans}`;
    ctx.fillText(d.number ? "SOUL NUMBER" : "STATUS", 94, 290);
    ctx.fillStyle = "#fff";
    ctx.shadowColor = "rgba(14,48,57,0.35)";
    ctx.shadowBlur = 24;
    ctx.shadowOffsetY = 8;
    ctx.font = `700 ${d.number ? 190 : 150}px ${f.display}`;
    ctx.fillText(d.number ? `#${String(d.number).padStart(4, "0")}` : STATUS_LABEL[d.status], 86, 450);
    ctx.shadowColor = "transparent";
    ctx.shadowBlur = 0;
    ctx.shadowOffsetY = 0;

    ctx.fillStyle = "#0e3039";
    ctx.font = `700 52px ${f.display}`;
    ctx.fillText(d.username ? `@${d.username}` : "Whitelisted soul", 94, 535);
    ctx.font = `500 26px ${f.sans}`;
    ctx.fillStyle = "rgba(14,48,57,0.75)";
    ctx.fillText(d.wallet, 96, 580);

    const c = STATUS_COLORS[d.status] || STATUS_COLORS.pending;
    ctx.font = `700 38px ${f.sans}`;
    const label = STATUS_LABEL[d.status] || STATUS_LABEL.pending;
    const pw = ctx.measureText(label).width + 80;
    rr(ctx, 94, 625, pw, 76, 38);
    ctx.fillStyle = c.bg;
    ctx.fill();
    ctx.lineWidth = 4;
    ctx.strokeStyle = "#0e3039";
    ctx.stroke();
    ctx.fillStyle = c.fg;
    ctx.textBaseline = "middle";
    ctx.fillText(label, 94 + 40, 625 + 40);
    ctx.textBaseline = "alphabetic";
    ctx.font = `500 24px ${f.sans}`;
    ctx.fillStyle = "rgba(14,48,57,0.75)";
    ctx.fillText(c.sub, 94, 740);

    ctx.fillStyle = "#fff0df";
    ctx.font = `600 italic 34px ${f.display}`;
    ctx.fillText("Drink. Forget. Reborn.", 94, CARD_H - 48);
    ctx.textAlign = "right";
    ctx.font = `500 26px ${f.sans}`;
    ctx.fillStyle = "#79d3df";
    ctx.fillText(`${d.host}   ·   @LetheansNFT`, CARD_W - 94, CARD_H - 50);
    ctx.textAlign = "left";
  }

  window.LetheansCard = { drawCard, cardCaption, CARD_W, CARD_H, CARD_SCALE };
})();
