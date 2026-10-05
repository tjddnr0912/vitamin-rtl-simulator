module sub #(parameter [64:0] P = 0) (); endmodule
module G1;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam int L = i;
    localparam [64:0] LW = i;
    wire [i:0] w;
    sub #(.P(i)) u ();
    if (i == 1) begin : c1 initial #0 $display("g[%0d] gen-if hit", i); end
    initial begin
      #0;
      $display("i=%0d L=%0d LW=%0d sel=%b bits_i=%0d bits_w=%0d uP=%0d plus=%0d", i, L, LW, i[0], $bits(i), $bits(w), u.P, i + 1);
    end
  end
  initial begin #1 $display("after: i=%h bits=%0d sel=%b", i, $bits(i), i[64]); #1 $finish; end
endmodule
