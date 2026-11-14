import { act } from "react";

type Check = (condition: unknown, message: string) => void;
type Sample = { width: number; height: number; top: number; radii: number[]; round: boolean };

/** Sample the real CSS transition, including a reversal while it is still running. */
export async function checkCapsuleMotion(check: Check, settle: (ms: number) => Promise<void>) {
  const island = document.querySelector<HTMLElement>(".island")!;
  const read = (): Sample => {
    const box = island.getBoundingClientRect();
    const css = getComputedStyle(island);
    return {
      width: box.width, height: box.height, top: box.top,
      radii: [css.borderTopLeftRadius, css.borderTopRightRadius, css.borderBottomLeftRadius, css.borderBottomRightRadius].map(parseFloat),
      round: ["round", "superellipse(1)"].includes(css.getPropertyValue("corner-shape").trim()) || !CSS.supports("corner-shape", "round"),
    };
  };
  const trigger = async (selector: string) => {
    await act(async () => { document.querySelector<HTMLElement>(selector)!.click(); });
  };
  const sample = async (ms: number) => {
    const frames: Sample[] = [];
    await act(async () => {
      await new Promise<void>((resolve) => {
        const start = performance.now();
        const frame = () => {
          frames.push(read());
          if (performance.now() - start < ms) requestAnimationFrame(frame);
          else resolve();
        };
        requestAnimationFrame(frame);
      });
    });
    return frames;
  };
  const collapsed = read();
  await trigger(".island__open-button");
  const opening = await sample(600);
  const expanded = read();
  const symmetric = (f: Sample) => f.round && f.radii.every((r) => Math.abs(r - f.radii[0]) < 0.01 && r > 0);
  check(opening.every(symmetric), "展开的每一帧四角对称且保持圆弧，没有顶部拍平或曲线跳变");
  check(opening.every((f) => Math.abs(f.top - collapsed.top) < 0.1), "展开过程中上缘位置稳定");
  check(opening.every((f, i) => f.height <= expanded.height + 0.5 && (i === 0 || f.height >= opening[i - 1].height - 0.5)), "展开高度单调到达面板，没有回弹过冲");
  check(Math.abs(expanded.radii[0] - 30 * expanded.width / 560) < 0.05 && expanded.radii[0] < expanded.height / 2, "展开为30 DIP圆角面板，不成为长椭圆");
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
    check(getComputedStyle(island).transitionDuration.split(",").every((value) => parseFloat(value) === 0), "减少动态效果时外壳不播放尺寸与圆角过渡");
  } else {
    check(opening.some((f) => f.height > collapsed.height + 1 && f.height < expanded.height - 1), "捕获到展开中间帧，尺寸连续过渡");
  }
  await trigger(".island__collapse-target");
  const closing = await sample(100);
  await trigger(".island__open-button");
  const reversing = await sample(100);
  await trigger(".island__collapse-target");
  const finalClose = await sample(600);
  const all = [...closing, ...reversing, ...finalClose];
  check(all.every((f) => symmetric(f) && Math.abs(f.top - collapsed.top) < 0.1 && f.height >= collapsed.height - 0.5 && f.height <= expanded.height + 0.5), "快速收起、反向展开、再次收起仍保持轮廓与上缘稳定");
  check(Math.abs(read().height - collapsed.height) < 0.1 && read().radii[0] === 19, "连续切换后完整回到38 DIP胶囊");
  // Native attachment events must not resurrect the old flat-top rules.
  document.documentElement.dataset.islandTopEdge = "attached";
  await settle(20);
  check(read().radii.every((r) => r === 19), "即使原生报告贴顶也不改变胶囊轮廓");
}
