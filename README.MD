# CoDex Pallet Documentation

 ![image](images/2.png)

The CoDEX Pallet Project is a Decentralized Exchange designed for the Substrate blockchain framework. 
It enables the creation and management of liquidity pools for asset pairs, allowing users to trade different digital assets efficiently. 
The platform supports key functionalities such as initializing pools with specified asset pairs, adding or removing liquidity, and conducting asset swaps while maintaining liquidity balance. 
It employs a constant product formula to ensure liquidity and price stability. 
Additionally, the project incorporates a variety of error checks and safeguards to ensure smooth and secure operations. 

## Fee calculation

The dex incorporates a fee. Currently the fee logic is very simple. Each swap holds the fee from the input amount 


## Extrinsics Overview
The DEX pallet's extrinsics are designed to interact with liquidity pools on the blockchain, enabling the creation, management, and usage of these pools. Each extrinsic performs specific roles and has built-in checks to ensure transactions are valid and secure.

### initialise_pool_with_assets
#### Purpose: 
To create a new liquidity pool with two distinct assets. This extrinsic is the starting point for setting up a market for asset exchange.
#### Checks:
Ensures the assets are distinct.

Validates that the provided asset amounts are non-zero.

Confirms the creator's account balance is sufficient for both assets.

Verifies that the pool for the given asset pair doesn't already exist.

### add_liquidity

#### Purpose: 

To contribute additional assets to an existing liquidity pool. This is essential for users wanting to increase their share in a pool or improve the pool's depth.

#### Checks:

Confirms that the pool exists for the given asset pair.

Ensures the assets are distinct and the amounts are non-zero.

Verifies that the user’s balance is sufficient for the contribution.

#### Usage: 

Utilized by liquidity providers to enhance their stake in a pool, thereby increasing its total liquidity.

### remove_liquidity
#### Purpose: 

To withdraw a portion or all of a user's stake from a liquidity pool. This allows users to retrieve their assets plus any accrued fees.

#### Checks:

Validates the existence of the pool for the specified asset pair.

Confirms the user has enough liquidity tokens for the withdrawal.

Ensures the amounts received are not less than the specified minimums, guarding against excessive slippage.

#### Usage: 
Employed by users when they want to exit the pool or reclaim a portion of their assets.

### swap_assets
#### Purpose: 
To exchange one asset for another within a pool. This function is core to the DEX's functionality, enabling users to trade assets.
#### Checks:
Ensures the input and output assets are different and part of the same pool.

Confirms the user has enough of the input asset.

Validates that the output amount is not less than the user's minimum expectation.

#### Usage: 
Central to traders looking to exchange assets at prevailing market rates determined by the liquidity pool.


# Future Improvements
- Handling Large Pools: The current implementation uses a k coefficient in the swap calculations. As the pool size increases, there's a risk of overflowing the maximum value of k, potentially locking the swap functionality. Advanced mathematical logic from Substrate could be utilized to mitigate this issue.
- Precision in Calculations: Swaps and liquidity calculations use integer division, which may lead to precision loss, particularly with small asset amounts. Incorporating higher-precision mathematics (e.g., using units like DOT & Plank, ETH & WEI) could enhance accuracy.
- Adjustable Fee Structure: The current implementation has a constant fee. Future iterations could introduce a configurable or market-condition-dependent fee structure, adapting to different economic scenarios.
- Providing mechanisms to mitigate impermanent loss for liquidity providers could attract more users. This could involve compensatory mechanisms or innovative pool designs that reduce the impact of price divergence.
- Incorporating risk assessment tools, like price oracles or automated auditing systems, could help users make informed decisions and enhance platform security.
