import {WagmiAdapter} from '@reown/appkit-adapter-wagmi';
import {arbitrum, type AppKitNetwork} from '@reown/appkit/networks';

export const projectId = 'c57e0bb001a4dc96b54b9ced656a3cb8';

export const networks: [AppKitNetwork, ...AppKitNetwork[]] = [arbitrum];

export const wagmiAdapter = new WagmiAdapter({
    networks,
    projectId,
});
