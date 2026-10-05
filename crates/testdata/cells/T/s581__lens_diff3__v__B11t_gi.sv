module top;
  typedef logic [39:0] t40;
  function automatic t40 ft40(input int a); return 40'h10_0000_0000 + a; endfunction
  if (ft40(12) ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
