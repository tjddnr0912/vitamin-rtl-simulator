module top;
  logic a, b;
  wire [3:0] arr;
  function automatic integer idx_f(input logic x); $display("idx t=%0t x=%b", $time, x); return x ? 1 : 0; endfunction
  assign arr[idx_f(a)] = b;
  initial begin a = 1'b0; b = 1'b1; end
  initial #1 $display("t1 arr=%b", arr);
  initial #10 $finish;
endmodule
