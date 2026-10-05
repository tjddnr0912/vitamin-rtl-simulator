package pk;
  function automatic logic [39:0] g40(input int a); return 40'h10_0000_0000 + a; endfunction
  localparam logic [39:0] PKA [2] = '{40'h10_0000_000C, 40'h0};
endpackage

module top;

  if (pk::PKA[0] ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
