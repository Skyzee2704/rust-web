//! Project facts, lore, quests, and sample art. Edit here; the pages read these values.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskKind {
    Follow,
    Like,
    Retweet,
    JoinDiscord,
    Custom,
}

#[derive(Clone, Copy, Debug)]
pub struct Task {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub kind: TaskKind,
    pub url: &'static str,
    pub required: bool,
    pub requires_link: bool,
    pub target_id: Option<&'static str>,
}

pub struct Site {
    pub name: &'static str,
    pub tagline: &'static str,
    pub description: &'static str,
    pub share_host: &'static str,
    pub supply: u32,
    pub chain: &'static str,
    pub mint_date: &'static str,
    pub mint_price: &'static str,
    pub wl_price: &'static str,
    pub phases: &'static [&'static str],
    pub x_handle: &'static str,
    pub x_url: &'static str,
    pub opensea: &'static str,
}

pub const SITE: Site = Site {
    name: "Letheans",
    tagline: "Drink. Forget. Reborn.",
    description: "Letheans are small, wide-eyed souls reborn from the River Lethe at the edge of Elysium. A collectible PFP collection.",
    share_host: "letheans.xyz",
    supply: 2222,
    chain: "Elysium",
    mint_date: "",
    mint_price: "TBA",
    wl_price: "TBA",
    phases: &["GTD", "FCFS", "Public"],
    x_handle: "LetheansNFT",
    x_url: "https://x.com/LetheansNFT",
    opensea: "",
};

pub struct Legend {
    pub name: &'static str,
    pub image: &'static str,
    pub line: &'static str,
    pub traits: &'static [&'static str],
    pub color: &'static str,
}

pub const LEGENDS: &[Legend] = &[
    Legend {
        name: "Fortune",
        image: "/nft/sample-1.webp",
        line: "Forgot every debt. Kept a halo of gold coins.",
        traits: &["Golden skin", "Coin halo", "White robe"],
        color: "#ebbc71",
    },
    Legend {
        name: "Hush",
        image: "/nft/sample-2.webp",
        line: "Woke up blue and quiet. Still trying to whistle.",
        traits: &["Blue skin", "Brown hair", "Black hoodie"],
        color: "#dcca58",
    },
    Legend {
        name: "Ranger",
        image: "/nft/sample-3.webp",
        line: "Crossed the river in a cowboy hat. Still in shock.",
        traits: &["Orange hair", "Cowboy hat", "Red shirt"],
        color: "#db5f57",
    },
    Legend {
        name: "Jade",
        image: "/nft/sample-4.jpg",
        line: "Rose from the river in techwear. Nobody asked questions.",
        traits: &["Teal eyes", "Techwear", "Jade flames"],
        color: "#17b39a",
    },
];

pub const BASE_IMAGE: &str = "/nft/base.png";

pub fn unrevealed() -> u32 {
    SITE.supply - LEGENDS.len() as u32
}

pub struct Step {
    pub title: &'static str,
    pub body: &'static str,
}

pub const STEPS: &[Step] = &[
    Step {
        title: "Connect X",
        body: "Step up to the river and show us who you are.",
    },
    Step {
        title: "Finish the quests",
        body: "Follow, like & repost, and reply on X. Done quests turn green.",
    },
    Step {
        title: "Drop a wallet",
        body: "Name the wallet that will carry your second life.",
    },
    Step {
        title: "Wait for the judgment",
        body: "Results are announced on X before the mint. Only the worthy get to drink.",
    },
];

pub const MARQUEE: &[&str] = &[
    "Drink from the river",
    "Forget everything",
    "Reborn in Elysium",
    "2222 souls",
    "A second life",
    "New faces, no names",
];

pub struct Chapter {
    pub title: &'static str,
    pub body: &'static [&'static str],
}

pub const LORE: &[Chapter] = &[
    Chapter {
        title: "Elysium",
        body: &["At the western edge of the world, on the shore of Oceanus, lies a meadow that has never known winter: Elysium. There, spring is eternal, and souls judged worthy rest after the battle of their lives."],
    },
    Chapter {
        title: "The River Lethe",
        body: &["But Elysium has one last door: the River Lethe, the river of forgetting. A soul that wishes to be reborn must drink from it and forget everything. Its wars. Its losses. Its own name."],
    },
    Chapter {
        title: "Not every soul may drink",
        body: &["Only those once judged worthy by the judge are given a second life. The rest wander the Asphodel Fields, ordinary forever. Or fall into Tartarus, and never return."],
    },
    Chapter {
        title: "What rises from the water",
        body: &["What rises from the river is no longer an old hero. They rise as Letheans: small souls, bald and wide-eyed. They are born blank. No memory, no clothes, no name."],
    },
    Chapter {
        title: "The First",
        body: &["The first to rise carried a sparkle on his head and a star in his eyes: the mark of the First, number one, the brightest soul. Only he bears it."],
    },
    Chapter {
        title: "A new face",
        body: &[
            "Their new identity is built from what they wear: hats, eyes, outfits. A Lethean may have laser eyes, a cowboy hat, a rocket backpack, or diamond hands. They forgot the past, but they still remember how to joke.",
            "The Letheans are the few who came back with new faces and a second life.",
        ],
    },
];

pub const LORE_MOTTO: &[&str] = &["Drink.", "Forget.", "Reborn."];

pub struct Faq {
    pub q: &'static str,
    pub a: &'static str,
}

pub const FAQS: &[Faq] = &[
    Faq {
        q: "What are Letheans?",
        a: "Letheans are small, wide-eyed souls reborn from the River Lethe at the edge of Elysium. They are a collectible PFP collection of 2,222 unique characters.",
    },
    Faq {
        q: "What chain is the collection on?",
        a: "Elysium. The total supply is 2,222.",
    },
    Faq {
        q: "When is the mint and how much does it cost?",
        a: "The mint date and prices are still to be announced. Everything will be posted on our X first, so follow @LetheansNFT and turn on notifications.",
    },
    Faq {
        q: "How do I get on the whitelist?",
        a: "Open the Apply page, connect your X account, finish the quests (follow, like & repost, reply and tag 3 friends), then drop your wallet address. Results are announced on X before the mint.",
    },
    Faq {
        q: "Which wallet should I use?",
        a: "Any EVM wallet address that starts with 0x. Use a wallet you control and double-check the address before submitting: each X account and each wallet can only apply once.",
    },
    Faq {
        q: "Do I really need an X account?",
        a: "Yes. You must sign in with X to apply, and the quests are checked against your account. Applications without a real X login are not accepted.",
    },
    Faq {
        q: "What are the mint phases?",
        a: "The plan is three phases: GTD, FCFS and Public. Spots per phase and who qualifies for each will be announced on X before the mint.",
    },
    Faq {
        q: "What will the NFTs look like?",
        a: "Every Lethean is a unique, hand-made character with its own mix of background, skin, expression, outfit and more. We'll share more of the collection as the mint gets closer.",
    },
    Faq {
        q: "How do I avoid scams?",
        a: "Our only official account is @LetheansNFT on X. We will never ask for your seed phrase or private key, and we never DM you first with mint links. If in doubt, don't connect your wallet.",
    },
];

pub const TASKS: &[Task] = &[
    Task {
        id: "follow-x",
        title: "Follow @LetheansNFT on X",
        description: "Stay on the riverbank — follow the official account.",
        kind: TaskKind::Follow,
        url: "https://x.com/LetheansNFT",
        required: true,
        requires_link: false,
        target_id: None,
    },
    Task {
        id: "like-rt-post",
        title: "Like & repost the WL post",
        description: "Like and repost the pinned whitelist post.",
        kind: TaskKind::Retweet,
        url: "https://x.com/REPLACE_ME/status/REPLACE_ME",
        required: true,
        requires_link: false,
        target_id: None,
    },
    Task {
        id: "reply-tag-3",
        title: "Reply & tag 3 friends",
        description:
            "Reply to the WL post, tag 3 friends, then paste the link to your reply below.",
        kind: TaskKind::Custom,
        url: "https://x.com/LetheansNFT",
        required: true,
        requires_link: true,
        target_id: None,
    },
];

pub fn required_task_ids() -> Vec<&'static str> {
    TASKS.iter().filter(|t| t.required).map(|t| t.id).collect()
}

pub fn task_by_id(id: &str) -> Option<&'static Task> {
    TASKS.iter().find(|t| t.id == id)
}

pub const APPLY_HOW: &[&str] = &[
    "Connect the X account you actually use.",
    "Open each quest, do it, then tick it off.",
    "Drop an EVM wallet you control.",
    "Results are announced on X before the mint.",
];
