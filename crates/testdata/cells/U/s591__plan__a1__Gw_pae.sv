module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction
    localparam integer W = fae(i);
    initial #3 $display("pae %m W=%0d", W);
  end
  initial #100 $finish;
endmodule
