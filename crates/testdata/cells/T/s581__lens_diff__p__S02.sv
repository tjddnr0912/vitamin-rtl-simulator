module top;
  function automatic int f(input logic [7:0] a);
    logic [3:0] t; t = a[3:0];
    return (t ==? 4'b1?00) + 2*(a ==? 8'b1???_????) + 4*(a inside {8'b0000_11??}) + 8*((t + 4'd4) ==? 5'b1_0???) + 16*((a + 8'd1) inside {9'b1_0000_000?});
  endfunction
  localparam L1 = f(8'hCC), L2 = f(8'h0C), L3 = f(8'h8D), L4 = f(8'hFF);
  initial $display("@F %0d %0d %0d %0d", L1, L2, L3, L4);
  initial $display("@R %0d %0d %0d %0d", f(8'hCC), f(8'h0C), f(8'h8D), f(8'hFF));
endmodule
