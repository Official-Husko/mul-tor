import requests
import os
import random
import json

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "uDrop"

"""
For some weird unholy reason this script stopped working. it works fine in insonmnia tho? what the fuck..
"""

"""

    "uDrop": {
        "apiKey": False,
        "url": "https://udrop.com/ajax/file_upload_handler",
        "api_url": "https://udrop.com/",
        "download_url_base": "https://udrop.com/",
        "size_limit": 10,
        "size_unit": "GB"
    },

"""

class uDrop:
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
        raw_req = "None :("
        try:
            # Select a random user agent
            ua = random.choice(user_agents)

            # Assemble Size Limit e.g. 5 GB
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'

            # Get the file size and name
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Calculate the size unit for the site
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":

                chunk_size = 500000000  # 500 MB
                chunk_position_before = -1
                chunk_position_after = -1

                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        chunk_length = len(chunk_data)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        
                        chunk_position_before = chunk_position_after + 1
                        chunk_position_after = chunk_position_before + chunk_length - 1

                        # Set the user agent header
                        headers = {
                            "User-Agent": ua,
                            "Accept": "application/json",
                            "X-Requested-With": "XMLHttpRequest",
                            "Content-Range": f"bytes {chunk_position_before}-{chunk_position_after}/{file_size}"
                        }

                        upload_data = {
                            "maxChunkSize": chunk_size,
                            "uploadSource": "file_manager",
                        }

                        # Prepare the json data to add extra data to the upload
                        form_data = {
                                    'files[]': (os.path.basename(file), chunk_data, 'application/octet-stream')
                                }

                        # Get the upload URL from the site data dictionary
                        upload_url = sites_data_dict[site]["url"]

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.post(url="https://udrop.com/ajax/file_upload_handler", data=upload_data, files=form_data, headers=headers, proxies=proxies, timeout=300, stream=True)

                raw_req = raw_req.json()
                download_url = raw_req[0].get("url", "")

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

