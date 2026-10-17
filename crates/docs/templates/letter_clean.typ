// Waypoint "Clean" cover letter. Same header as the Clean resume, one page, real text.
#let d = json("/data.json")

#set document(title: d.name + " — Cover letter", author: d.name)
#set page(paper: if d.at("paper", default: "a4") == "us-letter" { "us-letter" } else { "a4" }, margin: (x: 2.2cm, top: 1.5cm, bottom: 1.8cm))
#set text(font: "Inter", size: 10.4pt, fill: rgb("#1d1d1f"), lang: d.at("lang", default: "en"))
#set par(leading: 0.68em, spacing: 1.05em, justify: false)
#set list(indent: 0.2em, body-indent: 0.6em, spacing: 0.7em, marker: text(fill: rgb("#86868b"))[•])

#let muted(body) = text(fill: rgb("#6e6e73"), body)

#text(size: 20pt, weight: "semibold", tracking: -0.01em, d.name)
#if d.contact.len() > 0 { v(-0.5em); muted(text(size: 9.6pt, d.contact.join("   |   "))) }
#v(-0.2em)
#line(length: 100%, stroke: 0.5pt + rgb("#d2d2d7"))
#v(1.2em)

#grid(columns: (1fr, auto), gutter: 1em,
  d.recipient.join(linebreak()),
  muted(d.date))
#v(1.4em)

#if d.subject != "" { text(weight: "semibold", d.subject); v(0.3em) }
#if d.salutation != "" { d.salutation }

#if d.opening != "" { d.opening }

#if d.highlights.len() > 0 { list(..d.highlights) }

#for p in d.body { par(p) }

#v(0.6em)
#d.closing
#v(1.6em)
#text(weight: "medium", d.signature)
