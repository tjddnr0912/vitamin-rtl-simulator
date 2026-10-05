module top;
  typedef logic [39:0] t40;
  localparam t40 PTF = 40'h0C;
  if (PTF ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
