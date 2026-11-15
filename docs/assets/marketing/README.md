# James campaign artwork

These images were generated for the README on 2026-11-15 using OpenAI's built-in image-generation tool. They are **concept artwork**, not screenshots, interface specifications, or validation evidence. Both are supplied under the repository's [MIT license](../../../LICENSE).

| Asset | Purpose | Dimensions | Format |
| --- | --- | --- | --- |
| [james-hero.webp](james-hero.webp) | Light product introduction with the lavender companion | 1672 × 941 | WebP, quality 90 |
| [james-inspect.webp](james-inspect.webp) | Dark editorial panel about inspectable context | 1672 × 941 | WebP, quality 90 |

Only file encoding changed after generation. No generated interface is presented as shipped software. Existing application screenshots in the parent directory are separate, unedited captures; see [source provenance](../../../PROVENANCE.md).

The campaign uses generous spacing, graphite and warm-white backgrounds, large typography, and the companion's lavender accent. The user requested an Apple-style presentation; these are original James compositions, with no Apple logo or product imagery and no implied affiliation.

## Generation prompts

### Product hero

```text
Use case: ads-marketing.
Create a finished premium product-launch hero banner for James, an open-source desktop AI companion by Vers3Dynamics. Landscape 16:9, approximately 2400x1350. Apple-style editorial restraint, sophisticated industrial-design campaign photography, spectacularly clean typography, generous negative space, believable studio lighting. Original James branding only; no Apple logo, no Apple device, no endorsement.
Backdrop: seamless warm-white to extremely pale silver studio cyclorama, subtle soft ground shadow. Left 48% is typography, right 52% is a beautifully crafted lavender octopus mascot rendered as a satin ceramic collectible sculpture. It has a rounded smooth mantle, two small white eyes with charcoal pupils, exactly eight separate elegantly curved short arms, warm intelligent calm expression, no mouth. It should feel like a quiet desk companion: sculptural and refined, not a busy cartoon. Muted lavender (#997dc7), restrained violet highlights, flawless but physically believable satin material. Shot with a premium medium-format product camera, front three-quarter angle, soft large area light. Octopus large but all arms fully within the composition, sitting on the studio floor. No pedestal, no UI, no screens, no charts, no random props, no particles, no floating icons, no lens flare, no excessive glow.
Text verbatim in crisp charcoal modern neo-grotesk typography, carefully typeset left, with plenty of margins:
Small brand at upper left: "VERS3DYNAMICS"
Large title: "James."
Large but smaller than title, below on two lines: "Intelligence." / "At your side."
Small subtitle below: "Your model. Your context. Your call."
No other text. Make the headline readable even when the image is displayed 850px wide. Premium publishing-ready composition. This is conceptual brand artwork, not an app screenshot.
```

### Inspectable context

```text
Use case: ads-marketing.
Create a second finished premium campaign banner for James, an open-source desktop AI companion by Vers3Dynamics. Landscape 16:9, approximately 2400x1350. Apple-style product storytelling with rigorous minimalist typography and extraordinary materials. Original brand artwork, no Apple logo, no device or app mockup.
Deep near-black graphite seamless studio background. Left 46% is oversized beautiful warm-white neo-grotesk typography in three deliberate lines, exact text:
"Ask."
"Inspect."
"Decide."
Small lavender eyebrow above: "JAMES"
Small warm-grey subtitle below the headline, exact: "A closer look at every answer."
Right 54% is an abstract physical sculpture symbolizing inspectable layers: three large thin, horizontal smoked-glass square slabs, slightly rounded corners, separated by small air gaps, floating in a precise elegant stack above a graphite tabletop. In each translucent slab, a few abstract short engraved lines suggest document text, but NO letters, numbers, icons or fake interface. Top slab slightly offset. Refined subtle violet light refracts along the glass edges, balanced by faint silver highlights. Premium macro product photography, dramatic controlled studio softboxes, physically realistic reflections, sharp beveled edges, understated texture, enough ambient visibility to see all slabs. One simple coherent object, strong scale and tactility. No sci-fi circuitry, arrows, network nodes, particles, busy backgrounds, neon clouds, shields, padlocks, technical charts, diagram claims, extra text or watermarks. Readable headline when displayed at 850px. This is conceptual marketing artwork about inspecting context, not technical evidence and not a screenshot.
```

## Web encoding

The generated PNGs were encoded without resizing or compositing:

```bash
magick hero-source.png -quality 90 james-hero.webp
magick inspect-source.png -quality 90 james-inspect.webp
```

At publication, the two WebP assets total approximately 205 KiB, compared with approximately 3.6 MiB for the generated PNGs. README alt text supplies the campaign wording and identifies each visual. Essential feature claims and download links remain readable Markdown outside the images.

