import ranim

"""
<module 'ranim' from 'H:\\_ranim\\ranim\\.venv\\Lib\\site-packages\\ranim\\__init__.py'>
<class 'builtins.Timeline'>
<builtins.Timeline object at 0x000002C278F2E600>
"""
# print(ranim)
# print(ranim.Timeline)
# print(ranim.Timeline())

timeline = ranim.Timeline()

with open("assets/Ghostscript_Tiger.svg") as f:
    svg = f.read()

svg = ranim.SvgItem(svg)

timeline.show(svg)
timeline.forward(1.0)

ranim.render_timeline(timeline, "./")