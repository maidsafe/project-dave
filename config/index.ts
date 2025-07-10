import {WagmiAdapter} from "@reown/appkit-adapter-wagmi";
import {arbitrum} from "@reown/appkit/networks";
import {injected} from "wagmi/connectors";

const connector = injected({
    shimDisconnect: false,
});

export const networks = [arbitrum];

export const projectId = "1229cfd3cf7355433bc9ab7f4ea9e97e";

export const wagmiAdapter = new WagmiAdapter({
    networks,
    projectId,
    connectors: [connector],
})
