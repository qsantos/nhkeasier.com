import logging
from logging.handlers import TimedRotatingFileHandler


def init_logging() -> None:
    logger = logging.getLogger()
    logger.setLevel(logging.DEBUG)

    formatter = logging.Formatter(
        '%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    )

    handler = logging.StreamHandler()
    handler.setLevel(logging.INFO)
    handler.setFormatter(formatter)
    logger.addHandler(handler)

    thandler = TimedRotatingFileHandler('nhkeasier.com.log', when='D')
    thandler.setLevel(logging.DEBUG)
    thandler.setFormatter(formatter)
    logger.addHandler(thandler)

    logger = logging.getLogger(__name__)
    logger.debug('Logging initialized')
