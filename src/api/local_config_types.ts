/** 存储目的地枚举 */
export type StorageDestination = "WebDAV" | "Local" | "OneDrive";

export type LocalConfig = {
    storage_destination: StorageDestination;
    local_save_config: LocalSaveConfig;
    webdav_save_config: WebDAVConfig;
    onedrive_save_config: OneDriveConfig;
    save_to_local_per_update: number;
}

export type LocalSaveConfig = {
    destination_dir: string;
}

export type WebDAVConfig = {
    host_url: string;
    account: string;
    password: string;
    destination_dir: string;
}

export type OneDriveConfig = {
    refresh_token: string;
    destination_dir: string;
}

export type LocalStorageInner = {
    remote_config_dir: string;
}

export type PartialLocalSaveConfig = Partial<LocalSaveConfig>

export type PartialWebDAVConfig = Partial<WebDAVConfig>

export type PartialOneDriveConfig = Partial<OneDriveConfig>

export type PartialLocalConfig = {
    storage_destination?: StorageDestination;
    local_save_config?: PartialLocalSaveConfig;
    webdav_save_config?: PartialWebDAVConfig;
    onedrive_save_config?: PartialOneDriveConfig;
    save_to_local_per_update?: number;
}
