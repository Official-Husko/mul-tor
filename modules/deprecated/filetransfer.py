import requests
import os
import random
import base64

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "FileTransfer"

"""
Last Checked 24/03/2024
"""

"""

"FileTransfer": {
    "apiKey": False,
    "url": "{server}",
    "api_url": "https://filetransfer.io/",
    "download_url_base": "https://filetransfer.io/data-package/",
    "server_url": "https://filetransfer.io/api/v1/upload",
    "initialize_url": "https://filetransfer.io/start-upload",
    "finalize_url": "{final_url}",
    "size_limit": 6,
    "size_unit": "GB"
},

"""

"""

Throws 415 errors even tho it should all be correct. wasted too much time for something this poorly coded. 
To whoever made this site and system go fuck yourself and do us all a favor and stop coding things.

~ An edit 3 months later. I was a bit angry yes.

"""

class FileTransfer:
     def Uploader(file, proxy_list, user_agents, api_keys):
        """
        Uploads a file to a specified site using random user agents and proxies.

        Args:
            file (str): The path to the file to be uploaded.
            proxy_list (list): A list of proxy URLs.
            user_agents (list): A list of user agent strings.

        Returns:
            dict: A dictionary containing the status, file name, file URL, and site.

        Raises:
            Exception: If an error occurs during the upload process.
        """
        try:
            # Select a random user agent
            ua = random.choice(user_agents)

            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'

            # Get the file size and name
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)

            # Get the upload URL and size limit from the site data dictionary
            initialize_url = sites_data_dict[site]["initialize_url"]

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Calculate the size unit for the site
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            # Set the user agent header
            headers = {
                "User-Agent": ua,
                "X-Requested-With": "XMLHttpRequest"
            }

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                # Send the upload request with the form data, headers, and proxies
                init_req = requests.get(url=initialize_url, headers=headers, proxies=proxies)

                # Parse the response JSON and get the download URL
                init_resp = init_req.json()
                server_url = init_resp.get("uploadUrl", "")

                base64_filename = base64.b64encode(file_name.encode('utf-8'))
                base64_filename = base64_filename.decode('utf-8')
                print(base64_filename)

                headers = {
                    "User-Agent": ua,
                    "Tus-Resumable": "1.0.0",
                    "Upload-Length": f"{file_size}",
                    "Upload-Metadata": f"filename {base64_filename},filetype ,fileorder MQ==",
                }

                server_req = requests.post(url=server_url, headers=headers, proxies=proxies)

                print(server_req.status_code)
                print(server_req.headers)

                parts = server_url.split("/")
                print(parts)
                upload_url_start = parts[2]
                print(upload_url_start)

                server_location = server_req.headers["Location"]
                upload_url = f"https://{upload_url_start}{server_location}"
                print(upload_url)

                chunk_size = 10485760  # 10 MB (WHO THE FUCK DOES THIS SHIT IN 10MB CHUNKS? THIS WEBSITE IS FUCKED)
                chunk_offset = 0

                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        
                        base64_filename = base64.b64encode(file_name.encode('utf-8'))

                        headers = {
                            "User-Agent": ua,
                            "Tus-Resumable": "1.0.0",
                            "Upload-Offset": f"{chunk_offset}"
                        }
                        print(headers)

                        form_data = {
                                    'file': (os.path.basename(file), chunk_data, 'application/offset+octet-stream')
                                }

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.patch(url=upload_url, files=form_data, headers=headers, proxies=proxies, timeout=300)

                        print(raw_req.status_code)

                        if raw_req.status_code == 200 or raw_req.status_code == 204:
                            print("chunk uploaded")

                        chunk_offset += chunk_size

                headers = {
                    "User-Agent": ua,
                    "X-Requested-With": "XMLHttpRequest"
                }

                raw_req = requests.post(url=upload_url, headers=headers, proxies=proxies, timeout=300)
                print(raw_req.text)

                upload_resp = raw_req.json()

                download_url = upload_resp.get("deliveryPublicLink", "")

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

