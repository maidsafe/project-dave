import {WagmiAdapter} from "@reown/appkit-adapter-wagmi";
import {arbitrum} from "@reown/appkit/networks";
import {injected} from "wagmi/connectors";

const connector = injected({
    shimDisconnect: true,
});

export const networks = [arbitrum];

export const projectId = "c57e0bb001a4dc96b54b9ced656a3cb8";

export const wagmiAdapter = new WagmiAdapter({
    networks,
    projectId,
    connectors: [connector],
})
