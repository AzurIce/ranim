import ranimpy

"""
<module 'ranim' from 'H:\\_ranim\\ranim\\.venv\\Lib\\site-packages\\ranim\\__init__.py'>
<class 'builtins.Timeline'>
<builtins.Timeline object at 0x000002C278F2E600>
"""
# print(ranim)
# print(ranim.Timeline)
# print(ranim.Timeline())

def timeline_test() -> ranimpy.Timeline:
    timeline = ranimpy.Timeline()

    with open("../../assets/Ghostscript_Tiger.svg") as f:
        svg = f.read()

    svg = ranimpy.SvgItem(svg)
    svg.shift((100, 100, 0))

    timeline.show(svg)
    timeline.forward(1.0)

    return timeline

if __name__ == '__main__':
    ranimpy.render_timeline(timeline_test(), "./")