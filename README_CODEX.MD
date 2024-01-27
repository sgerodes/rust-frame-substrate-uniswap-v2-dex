swap_assets

Future improvements:
The swap uses the k coefficient. Currently if the pool gets too big the k coefficient can overflow the max value and the whole swap will be locked
This could be prevented by using some more advanced math logic that substrate provides.

The swaps and liquidity calculation use integer division, which can lead some precision loss, especially when dealing with small asset amounts.
Currently the user will just get less assets because of that, so this behaviour will not break the dex, 
but this could be improved in the future by incorporating math with better precision. 
E.g. DOT & Plank, ETH & WEI math


adjustable fee. Currently we use a contant fee. We could make it configurable, or even automatically adjustable to some market conditions.
