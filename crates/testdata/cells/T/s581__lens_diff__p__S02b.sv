module top;
  function automatic int f1(input logic [7:0] a); return (a ==? 8'b1???_????); endfunction
  function automatic int f2(input logic [7:0] a); return ((a + 8'd1) inside {9'b1_0000_000?}); endfunction
  function automatic int f3(input logic [3:0] t); return ((t + 4'd4) ==? 5'b1_0???); endfunction
  function automatic int f4(input logic signed [3:0] t); return (t ==? 8'sb1111_1?00) + 2 * (t !=? 8'b1111_1?00); endfunction
  function automatic int f5(input int n); int r; r = 0; for (int k = 0; k < n; k++) r = r + (k ==? 32'b????_????_????_????_????_????_????_?1?1); return r; endfunction
  localparam L1 = f1(8'hCC), L2 = f2(8'hFF), L3 = f3(4'hC), L4 = f4(-4'sd4), L5 = f5(16);
  initial $display("@F %0d %0d %0d %0d %0d", L1, L2, L3, L4, L5);
  initial $display("@R %0d %0d %0d %0d %0d", f1(8'hCC), f2(8'hFF), f3(4'hC), f4(-4'sd4), f5(16));
endmodule
