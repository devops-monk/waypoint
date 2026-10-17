// Waypoint "Modern" resume. Still one column of real text for ATS parsing, with the
// app's indigo as an accent on the name, section titles and the dates rail.
#let d = json("/data.json")
#let accent = rgb("#4a57e8")

#set document(title: d.name + " — Resume", author: d.name)
#set page(paper: if d.at("paper", default: "a4") == "us-letter" { "us-letter" } else { "a4" }, margin: (x: 1.6cm, top: 1.4cm, bottom: 1.4cm))
#set text(font: "Inter", size: 9.5pt, fill: rgb("#1d1d1f"), lang: "en")
#set par(leading: 0.56em)
#set list(indent: 0pt, body-indent: 0.6em, spacing: 0.5em, marker: text(fill: accent)[▸])

#let muted(body) = text(fill: rgb("#6e6e73"), body)
#let section(title) = {
  v(1em)
  grid(columns: (auto, 1fr), gutter: 0.6em, align: horizon,
    text(size: 10.5pt, weight: "bold", fill: accent, title),
    line(length: 100%, stroke: 0.6pt + accent.lighten(65%)))
  v(0.2em)
}

#grid(columns: (1fr, auto), align: bottom,
  [
    #text(size: 22pt, weight: "bold", fill: accent, tracking: -0.015em, d.name)
    #if d.headline != "" { v(-0.65em); text(size: 11pt, weight: "medium", d.headline) }
  ],
  align(right, muted(d.contact.join(linebreak()))))

#if d.summary != "" {
  section("Summary")
  d.summary
}

#if d.experience.len() > 0 {
  section("Experience")
  for (i, r) in d.experience.enumerate() {
    if i > 0 { v(0.6em) }
    block(breakable: true)[
      #grid(columns: (1fr, auto), gutter: 1em,
        [#text(weight: "semibold", r.title) #muted[at] #text(weight: "medium", r.company)],
        text(size: 8.8pt, fill: accent, weight: "medium")[#r.start – #r.end])
      #if r.location != "" { v(-0.55em); muted(text(size: 8.8pt, r.location)) }
      #if r.bullets.len() > 0 {
        v(-0.15em)
        list(..r.bullets)
      }
    ]
  }
}

#if d.skills.len() > 0 {
  section("Skills")
  d.skills.join("  ·  ")
}

#if d.education.len() > 0 {
  section("Education")
  for e in d.education {
    grid(columns: (1fr, auto), gutter: 1em,
      [#text(weight: "semibold", e.degree) #muted[—] #e.institution],
      muted[#e.start – #e.end])
    v(0.25em)
  }
}

#if d.languages.len() > 0 {
  section("Languages")
  d.languages.join("  ·  ")
}
