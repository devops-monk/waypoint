// Waypoint "Clean" resume. Single column, real text, standard section names: the layout
// applicant tracking systems parse most reliably.
#let d = json("/data.json")

#set document(title: d.name + " — Resume", author: d.name)
#set page(paper: if d.at("paper", default: "a4") == "us-letter" { "us-letter" } else { "a4" }, margin: (x: 1.7cm, top: 1.5cm, bottom: 1.4cm))
#set text(font: "Inter", size: 9.6pt, fill: rgb("#1d1d1f"), lang: "en")
#set par(leading: 0.55em, justify: false)
#set list(indent: 0pt, body-indent: 0.6em, spacing: 0.5em, marker: text(fill: rgb("#86868b"))[•])

#let muted(body) = text(fill: rgb("#6e6e73"), body)
#let section(title) = {
  v(0.9em)
  text(size: 10.4pt, weight: "semibold", title)
  v(-0.55em)
  line(length: 100%, stroke: 0.5pt + rgb("#d2d2d7"))
  v(0.15em)
}
#let dates(s, e) = if s != "" or e != "" { muted([#s – #e]) }

#text(size: 20pt, weight: "semibold", tracking: -0.01em, d.name)
#if d.headline != "" { v(-0.85em); text(size: 11pt, fill: rgb("#424245"), d.headline) }
#if d.contact.len() > 0 { v(-0.4em); muted(d.contact.join("   |   ")) }

#if d.summary != "" {
  section("Summary")
  d.summary
}

#if d.experience.len() > 0 {
  section("Experience")
  for (i, r) in d.experience.enumerate() {
    if i > 0 { v(0.55em) }
    block(breakable: true)[
      #grid(columns: (1fr, auto), gutter: 1em,
        [#text(weight: "semibold", r.title)#if r.company != "" [, #r.company]#if r.location != "" { muted[ · #r.location] }],
        dates(r.start, r.end))
      #if r.bullets.len() > 0 {
        v(-0.2em)
        list(..r.bullets)
      }
    ]
  }
}

#if d.skills.len() > 0 {
  section("Skills")
  d.skills.join(", ")
}

#if d.education.len() > 0 {
  section("Education")
  for e in d.education {
    grid(columns: (1fr, auto), gutter: 1em,
      [#text(weight: "semibold", e.degree)#if e.institution != "" [, #e.institution]],
      dates(e.start, e.end))
    v(0.25em)
  }
}

#if d.languages.len() > 0 {
  section("Languages")
  d.languages.join(", ")
}
