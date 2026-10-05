module top;

  if (4'd12 inside {4'b0?11, 4'b1?00}) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
