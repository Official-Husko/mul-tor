sites_data_dict = {
    "GoFile": {
        "apiKey": False,
        "url": "https://{server}.gofile.io/uploadFile",
        "api_url": "https://gofile.io/api/",
        "download_url_base": "https://gofile.io/d/",
        "server_url": "https://api.gofile.io/getServer"
    },
    "PixelDrain": {
        "apiKey": False,
        "url": "https://pixeldrain.com/api/file/{file_name}",
        "api_url": "https://pixeldrain.com/api/",
        "download_url_base": "https://pixeldrain.com/u/",
        "size_limit": 20,
        "size_unit": "GB"
    },
    "FileBin": {
        "apiKey": False,
        "url": "https://filebin.net/",
        "api_url": "https://filebin.net/",
        "download_url_base": "https://filebin.net/"
    },
    "Delafil": {
        "apiKey": False,
        "url": "https://delafil.se/core/page/ajax/file_upload_handler.ajax.php",
        "api_url": "https://delafil.se/",
        "download_url_base": "https://delafil.se/",
        "size_limit": 6,
        "size_unit": "GB"
    },
    "Files.dp.ua": {
        "apiKey": False,
        "url": "https://files2.dp.ua/upload.php",
        "api_url": "https://files2.dp.ua/upload.php",
        "download_url_base": "https://files.dp.ua/en/",
        "size_limit": 100,
        "size_unit": "GB"
    },
    "FilesFM": {
        "apiKey": False,
        "url": "https://free.files.fm/save_file.php?up_id={upload_id}&ignore_user_abort=1&skip_update=1&key={upload_key}",
        "api_url": "https://free.files.fm/save_file.php",
        "download_url_base": "https://files.fm/u/{upload_id}",
        "initialize_url": "https://files.fm/server_scripts/get_upload_id.php?show_add_key=1",
        "finalize_url": "https://free.files.fm/finish_upload.php?upload_hash={upload_id}",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "Krakenfiles": {
        "apiKey": False,
        "url": "{server}",
        "api_url": "https://krakenfiles.com/",
        "download_url_base": "https://krakenfiles.com",
        "server_url": "https://krakenfiles.com/api/server/available",
        "size_limit": 1,
        "size_unit": "GB"
    },
    "Transfer": {
        "apiKey": False,
        "url": "https://transfer.sh/{file_name}",
        "api_url": "https://transfer.sh/",
        "download_url_base": "https://transfer.sh/",
        "size_limit": 10,
        "size_unit": "GB"
    },
    "TmpFiles": {
        "apiKey": False,
        "url": "https://tmpfiles.org/api/v1/upload",
        "api_url": "https://tmpfiles.org/api",
        "download_url_base": "https://tmpfiles.org/",
        "size_limit": 100,
        "size_unit": "MB"
    },
    "MixDrop": {
        "apiKey": True,
        "url": "https://ul.MixDrop.ag/api",
        "api_url": "https://ul.MixDrop.ag/api",
        "download_url_base": "https://MixDrop.ag/f/",
        "size_limit": 10,
        "size_unit": "GB"
    },
    "1Fichier": {
        "apiKey": False,
        "url": "https://{server}/upload.cgi?id={upload_id}",
        "api_url": "https://api.1fichier.com/v1/upload/",
        "download_url_base": "https://1fichier.com/?",
        "initialize_url": "https://api.1fichier.com/v1/upload/get_upload_server.cgi",
        "finalize_url": "https://{server}/end.pl?xid={upload_id}",
        "size_limit": 300,
        "size_unit": "GB"
    },
    "Fileio": {
        "apiKey": False,
        "url": "https://file.io/?title={file_name}",
        "api_url": "https://file.io/",
        "download_url_base": "https://file.io/",
        "patch_url": "https://file.io/{file_id}",
        "size_limit": 2,
        "size_unit": "GB"
    },
    "EasyUpload": {
        "apiKey": False,
        "url": "https://upload{number}.easyupload.io/action.php",
        "api_url": "https://easyupload.io/",
        "download_url_base": "https://easyupload.io/",
        "initialize_url": "https://easyupload.io/action.php",
        "size_limit": 10,
        "size_unit": "GB"
    },
    "AnonTransfer": {
        "apiKey": False,
        "url": "https://anontransfer.com/upload.php",
        "api_url": "https://anontransfer.com/",
        "download_url_base": "https://anontransfer.com/download/",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "1CloudFile": {
        "apiKey": False,
        "url": "https://1cloudfile.com/ajax/file_upload_handler",
        "api_url": "https://1cloudfile.com/",
        "download_url_base": "https://1cloudfile.com/",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "AnonymFile": {
        "apiKey": False,
        "url": "https://anonymfile.com/upload",
        "api_url": "https://anonymfile.com/",
        "download_url_base": "https://anonymfile.com/",
        "size_limit": 7,
        "size_unit": "GB"
    },
    "FileSi": {
        "apiKey": False,
        "url": "https://file.si/api/v1/upload",
        "api_url": "https://file.si/",
        "download_url_base": "https://file.si/",
        "size_limit": 7,
        "size_unit": "GB"
    },
    "ClicknUpload": {
        "apiKey": False,
        "url": "{server}",
        "api_url": "https://clicknupload.click",
        "download_url_base": "https://clicknupload.vip/",
        "size_limit": 2,
        "size_unit": "GB"
    },
    "FileUpload": {
        "apiKey": False,
        "url": "https://up.file-upload.net/upload.php",
        "api_url": "https://file-upload.net/",
        "download_url_base": "https://file-upload.net/download-{file_id}/{server_name}.html",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "BowFile": {
        "apiKey": True,
        "url": "https://bowfile.com/api/v2/file/upload",
        "api_url": "https://bowfile.com/",
        "download_url_base": "https://bowfile.com/",
        "authorize_url": "https://bowfile.com/api/v2/authorize",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "HexUpload": {
        "apiKey": False,
        "url": "{server}",
        "api_url": "https://hexupload.net/",
        "download_url_base": "https://hexupload.net/",
        "size_limit": 2,
        "size_unit": "GB"
    },
    "UserCloud": {
        "apiKey": False,
        "url": "{server}",
        "api_url": "https://userscloud.com/",
        "download_url_base": "https://userscloud.com/",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "DooDrive": {
        "apiKey": True,
        "url": "{server}",
        "api_url": "https://doodrive.com/",
        "download_url_base": "https://doodrive.com/f/",
        "initialize_url": "https://doodrive.com/api/v1/upload",
        "finalize_url": "{server}",
        "size_limit": 2,
        "size_unit": "GB"
    },
    "uDrop": {
        "apiKey": False,
        "url": "https://www.udrop.com/ajax/file_upload_handler",
        "api_url": "https://www.udrop.com/",
        "download_url_base": "https://www.udrop.com/",
        "size_limit": 10,
        "size_unit": "GB"
    },
    "uFile": {
        "apiKey": False,
        "url": "{server}v1/upload/chunk",
        "api_url": "https://ufile.io/",
        "download_url_base": "https://ufile.io/",
        "initialize_url": "{server}v1/upload/create_session",
        "finalize_url": "{server}v1/upload/finalise",
        "server_url": "https://ufile.io/v1/upload/select_storage",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "DownloadGG": {
        "apiKey": False,
        "url": "https://download.gg/server/upload.php",
        "api_url": "https://download.gg/",
        "download_url_base": "https://download.gg/file-",
        "size_limit": 25,
        "size_unit": "GB"
    },
    "CatBox": {
        "apiKey": False,
        "url": "https://catbox.moe/user/api.php",
        "api_url": "https://catbox.moe/",
        "download_url_base": "https://files.catbox.moe/",
        "size_limit": 200,
        "size_unit": "MB",
    },
    "LitterBox": {
        "apiKey": False,
        "url": "https://litterbox.catbox.moe/resources/internals/api.php",
        "api_url": "https://litterbox.catbox.moe/",
        "download_url_base": "https://litter.catbox.moe/",
        "size_limit": 1,
        "size_unit": "GB"
    },
    "Keep": {
        "apiKey": False,
        "url": "https://free.keep.sh/",
        "api_url": "https://keep.sh/",
        "download_url_base": "https://free.keep.sh/",
        "size_limit": 500,
        "size_unit": "MB"
    },
    "TempSend": {
        "apiKey": False,
        "url": "https://tempsend.com/send",
        "api_url": "https://tempsend.com/",
        "download_url_base": "https://tempsend.com/",
        "size_limit": 2,
        "size_unit": "GB"
    },
    "UsersDrive": {
        "apiKey": False,
        "url": "{server}",
        "api_url": "https://usersdrive.com/",
        "download_url_base": "https://usersdrive.com/{file_id}.html",
        "size_limit": 2,
        "size_unit": "GB"
    },
    "Rapidgator": {
        "apiKey": True,
        "url": "{server}",
        "api_url": "https://rapidgator.net/",
        "download_url_base": "https://rapidgator.net/file/",
        "initialize_url": "https://rapidgator.net/api/v2/user/login",
        "server_url": "https://rapidgator.net/api/v2/file/upload",
        "finalize_url": "https://rapidgator.net/api/v2/file/upload_info",
    },
    "WDHO": {
        "apiKey": False,
        "url": "https://wdho.ru/core/page/ajax/file_upload_handler.ajax.php",
        "api_url": "https://wdho.ru/",
        "download_url_base": "https://wdho.ru/",
        "size_limit": 5,
        "size_unit": "GB"
    },
    "Filesadmin": {
        "apiKey": False,
        "url": "https://filesadmin.com/ajax/file_upload_handler",
        "api_url": "https://filesadmin.com/",
        "download_url_base": "https://filesadmin.com/",
        "size_limit": 3,
        "size_unit": "GB"
    },
    "Oshi": {
        "comment": "Keep this one as low as possible. Its really a slow piece of shit.",
        "apiKey": False,
        "url": "https://oshi.at/?shorturl=1",
        "api_url": "https://oshi.at/cmd",
        "download_url_base": "https://oshi.at/",
        "size_limit": 5,
        "size_unit": "GB"
    },
}

UNITS = {
    'B': 1,
    'KB': 1024,
    'MB': 1024 * 1024,
    'GB': 1024 * 1024 * 1024,
    'TB': 1024 * 1024 * 1024 * 1024,
    'PB': 1024 * 1024 * 1024 * 1024 * 1024,
}

class Site_Data_CLSS:
    def size_unit_calc(site_name, file_size):
        site_data = sites_data_dict.get(site_name)
        if site_data is None:
            raise ValueError(f"Site '{site_name}' not found in the data dictionary.")

        size_limit = site_data.get("size_limit", 0)
        size_unit = site_data.get("size_unit", "B")
        
        size_limit_bytes = size_limit * UNITS.get(size_unit.upper(), 1)

        if file_size > size_limit_bytes:
            return "SIZE_ERROR"
        return "OK"
