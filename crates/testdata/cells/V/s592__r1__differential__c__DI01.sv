interface ifc #(parameter W = 4) ();
  if (W == 8) begin : a logic [7:0] d = 8'hA5; end
  else begin : b logic [3:0] d = 4'h3; end
endinterface
module top;
  ifc #(.W(8)) i8();
  ifc #(.W(4)) i4();
  initial #1 $display("@i8 %h i4 %h", i8.a.d, i4.b.d);
  initial #10 $finish;
endmodule
