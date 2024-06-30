class FileSizeCalculator:
    """
    A class to calculate whether a file size exceeds a given limit.

    Attributes:
        site_name (str): The name of the site.
        file_size (int): The size of the file.
        size_limit (int): The size limit to compare against. e.g. 4
        size_unit (str): The unit of the size limit. e.g. GB
    """

    def __init__(self, site_name: str, file_size: int, size_limit_int: int, size_limit_str: str):
        """
        Initializes the FileSizeCalculator object.

        Args:
            site_name (str): The name of the site.
            file_size (int): The size of the file.
            size_limit (int): The size limit to compare against.
            size_unit (str): The unit of the size limit.
        """
        self.site_name = site_name
        self.file_size = file_size
        self.size_limit_int = size_limit_int
        self.size_unit_str = size_limit_str

        # Temporary variables
        self.byte_units: dict = self._unit_declaration()
        self.size_limit_bytes: int = 0

        self.status: str = self._calculate_file_size()

    def _unit_declaration(self):
        """
        Returns a dictionary with byte units as keys and their respective multipliers as values.

        Returns:
            dict: A dictionary mapping byte units to their multipliers.
        """
        return {
            'B': 1,
            'KB': 1024,
            'MB': 1024 * 1024,
            'GB': 1024 * 1024 * 1024,
            'TB': 1024 * 1024 * 1024 * 1024,
            'PB': 1024 * 1024 * 1024 * 1024 * 1024,
        }

    def _calculate_file_size(self):
        """
        Compares the file size with the size limit and returns the status.

        Returns:
            str: The status of the file size compared to the limit. OK or SIZE_ERROR.
        """
        self.size_limit_bytes = self.size_limit_int * self.byte_units.get(self.size_unit_str.upper(), 1)

        if self.file_size > self.size_limit_bytes:
            return "SIZE_ERROR"
        return "OK"
