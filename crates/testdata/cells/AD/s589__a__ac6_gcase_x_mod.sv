module top;
  function automatic logic [3:0] hm(input int x);
    logic [3:0] t;
    hm = t;
  endfunction
  case (4'd0)
    hm(2): begin : a initial #1 $display("arm=h"); end
    default: begin : d initial #1 $display("arm=def"); end
  endcase
  initial #2 $finish;
  initial #50 $finish;
endmodule
