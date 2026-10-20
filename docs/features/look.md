# The Halloween look

A look for the page, orange and black with drawings that move, for anyone who
wants it. It is off until you switch it on, it changes nothing about what
dbt-edith reads or shows, and it never reaches a file you export or a picture
you copy (0106, 0107, 0108).

## Switching it on

The pumpkin in the top bar, beside the shortcut list's `?`, switches the look
on and off. Once it is on, a speaker button appears beside it: sounds are muted
until you press it.

The settings have a Theme section with the same choices as three switches, the
look itself, the random apparitions and the sound, and a fourth, the style of
the lineage's boxes. The apparitions, the sound and the style apply once the
look is on, and the speaker in the top bar and the sound switch are one
choice. All four are kept on the machine, in `look.json` in
dbt-edith's config directory, so they hold for every project and every start;
where there is no config directory they last until the page is closed.

## What it changes

- **Colours**: the page turns near-black and the accent orange. The colours
  that carry a meaning, the kinds of node, the environments, a warning, a
  success or a failure, keep theirs, and the terminal is left as it is.
- **A ghost on Edith's E**, top left. It glows when a newer release is out, and
  at night, between 20:00 and 07:00 by your own clock.
- **A witch on her broom** flies over the dock's tabs to the one you open.
- **Zombie switches**: each switch in the settings becomes a zombie that walks
  to the other side when you flip it, facing the way it goes.
- **The lineage's boxes**, in the style the Theme section's Box style picks,
  the tombstone unless you pick another. Picking one redraws nothing:
  - **Tombstone**: a grey stone slab with an arched top, a chiselled edge, an
    engraved cross and a crack, the kind as moss along its base. Under the
    pointer the stone shudders, as if something pushed from underneath.
  - **Coffin**: dark wood with chamfered ends and a lid line, studded with
    pins, a gold cross at its head with the kind as its gem. Under the pointer
    it shakes and its lid lifts a little.
  - **Grimoire**: purple leather in a double gold border, a banded spine at
    the left with the kind as its gem, pages at the right. Under the pointer
    its cover lifts from the spine.

  The names are serif, and a long one ends in an ellipsis a little sooner than
  without the look; its hover card has the whole name. A box that is the focus,
  selected, a find match or drawn dashed (an ephemeral model) takes its ring on
  the silhouette itself. The tests under a model keep their usual shape. Edges
  become thin silk, the edges of the box you select turn orange, and a cobweb hangs in two
  corners of the canvas. The dashes that say an edge is a test, a back edge or
  indirect are kept, and those edges have no dewdrops.

  Where the boxes have their drawings, the silk carries small dewdrops, lighter
  in the hue of a role and orange on the edges of the box you selected. When
  you click a box, a small web grows on its top right corner, beside the box
  and not over it: threads, then arcs, then a spider that drops on its thread,
  in less than a second, again at each new selection. It is there with any style
  and on any size of canvas, and with reduced motion it simply appears (0108).

  The drawings come with up to 400 boxes on the canvas. A bigger selection
  gets a plainer version of the same style: the colour and the silhouette,
  without the cross, the plate, the spine or the rings following the shape
  (a ring is then a rounded rectangle), and the silk has no dewdrops. There
  the corners a silhouette cuts away take no click, which falls through to the
  canvas (0107).
- **Apparitions**: now and then a ghost crosses the page, every few minutes at
  random, and a pair of eyes blinks somewhere on the page a little less often.
  Neither takes a click, and none shows while the tab is hidden. The Theme
  section's Random apparitions switch turns them off, and any on screen goes at
  once. With the sound on, the ghost wails and the eyes whisper, sounds made in
  the browser as it goes; nothing is played, nor a sound device opened, while
  the look or its sound is off.

With reduced motion asked of the system, no box moves under the pointer, no ghost crosses, the
witch jumps rather than flies and the eyes fade without blinking.

## What it never touches

Export above the graph and Copy image draw the lineage as it is without the
look: the theme is scoped to the live page, so a file sent to a colleague or a
picture pasted in a ticket is the usual blue, in plain boxes.
