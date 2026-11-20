primitive p(q, a, b); output q; reg q; input a, b; initial q = 'x; table 0 ? : ? : 0; 1 ? : ? : 1; endtable endprimitive
module top; wire y; reg a, b; p u(y, a, b); endmodule
