// custom cursor when inside screen
// have another custom cursor outside of computer frame
// with trailing metaball paint trail follow pos

import React, { useEffect, useState } from 'react';
import './Cursor.css';

const Cursor = ({width, screenWidth}) => {
    const cursorWidth = 9;
    const [pos, setPos] = useState({ x: 0, y: 0 });

    useEffect(() => {
        const handleMove  = (e) => {
            setPos({ x : e.clientX, y: e.clientY });
        };
        window.addEventListener('mousemove', handleMove);
        return () => window.removeEventListener('mousemove', handleMove);
    }, []);

    return (
        <img
            src={process.env.PUBLIC_URL + "/assets/cursor/default.png"}
            alt="Cursor"
            style={{
                position: 'fixed',
                left: `${pos.x}px`,
                top: `${pos.y}px`,
                width: cursorWidth * (width / screenWidth) / 3, // adjust to your asset size
                // height: '12px'
                pointerEvents: 'none', // lets clicks pass through
                zIndex: 9999
            }}
        />
    )
}

export default Cursor;