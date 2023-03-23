# Usage within hUGETracker

fortISSimO can be used directly inside of hUGETracker, which lets you preview any [differences](./changes.md) live!
This only requires overwriting a couple of files, too.

> **DISCLAIMER**
>
> Using fortISSimO in hUGETracker is NOT officially supported by hUGETracker!
> If you get any issues (garbled sound, lockups, crashes) while using fortISSimO, **please report them to me**!
> hUGETracker will still open its own bug reporting page, but I can't change that.
>
> Those should be rare, but I can't rule out the possibility.

Here is how to "inject" fortISSimO into hUGETracker:

0. Locate the `hUGEDriver` directory next to the `hUGETracker.exe` you want to "mod"; we will be **overwriting** some files in there.
1. Copy `fortISSimO.asm` into that directory as `hUGEDriver.asm`.
2. Copy `fortISSimO.inc` into the `include` directory as `hUGE.inc`.
3. Modify `hUGEDriver.asm` (which is now just `fortISSimO.asm` in disguise).
   Look for the following line, at or near line 11:

   ```rgbasm
   ; def HUGETRACKER equs "1.0"
   ```

   1. Make sure between the quotes is the version of hUGETracker that you are using.
   2. Delete the `;`.

   You should end up with something like this:

   ```rgbasm
   def HUGETRACKER equs "1.0b10"
   ```

   If you forget to do this, you should get an error when you press the play button in hUGETracker.
4. You're done!

## Undoing the changes

To restore hUGEDriver, you simply need to restore the `hUGEDriver.asm` and `hUGE.inc` files you overwrote.
You can re-download hUGETracker, since the files are bundled with it.
