package q;
  function automatic logic [3:0] h(input int x);
    case (x)
      2: h = 4'd3;
      default: h = 4'd1;
    endcase
  endfunction
endpackage
module top;
  wire [31:0] w;
  assign w = {4'd0, {q::h(2){1'b1}}};
  initial begin #1 $display("w=%h", w); $finish; end
  initial #50 $finish;
endmodule
