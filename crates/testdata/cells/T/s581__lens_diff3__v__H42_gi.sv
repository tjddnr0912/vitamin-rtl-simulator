module top;

  if (4'd5 inside {65'b1?00, 4'b0?11}) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
