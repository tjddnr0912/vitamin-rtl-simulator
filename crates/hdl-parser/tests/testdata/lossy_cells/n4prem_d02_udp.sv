primitive p(output reg q, input a, input b); initial q = 'x; table 0 ? : ? : 0; 1 ? : ? : 1; endtable endprimitive
module top; wire y; reg a, b; p u(y, a, b); endmodule
