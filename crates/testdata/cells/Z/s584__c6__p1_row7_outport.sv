module top;
  wire [7:0] w;
  child u(.s(w));
  initial begin
    $display("r t=%0t w=%h", $time, w);
    #0 $display("r#0 t=%0t w=%h", $time, w);
    #0 $display("r#00 t=%0t w=%h", $time, w);
    #1 $display("e t=%0t w=%h", $time, w);
    $finish;
  end
endmodule
module child(output logic [7:0] s);
  always_comb s = 8'hEE;
endmodule
