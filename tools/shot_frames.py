"""Print one frame per shot (3/4 into it) as a comma list, from hud/src/generated.rs."""
import re
from pathlib import Path

s = (Path(__file__).resolve().parent.parent / "hud" / "src" / "generated.rs").read_text()
body = re.search(r"SHOT_FRAMES: \[usize; \d+\] = \[([^\]]*)\]", s, re.S).group(1)
a = [int(x) for x in re.findall(r"\d+", body)]
print(",".join(str(a[i] + max(1, (a[i + 1] - a[i]) * 3 // 4)) for i in range(len(a) - 1)))
