module top;
  localparam int W6 = 6;
  localparam logic [7:0] P8 = 8'hF0;
  localparam logic [5:0] XB = W6'(P8);
  case (6'h30) W6'(P8): begin : a initial $display("@A paramcast hit"); end default: begin : ad initial $display("@A paramcast def"); end endcase
  case (6'h30) 6'(P8): begin : b initial $display("@B litcast hit"); end default: begin : bd initial $display("@B litcast def"); end endcase
  case (6'h30) XB: begin : c initial $display("@C binder hit"); end default: begin : cd initial $display("@C binder def"); end endcase
  case (11'h783) {P8, 3'(W6'(3'h3))}: begin : e initial $display("@E concat hit"); end default: begin : ed initial $display("@E concat def"); end endcase
  case (-1) W6'(-1): begin : f initial $display("@F signedpass hit"); end default: begin : fd initial $display("@F signedpass def"); end endcase
  case (0) W6'(8'h40): begin : g initial $display("@G trunc hit"); end default: begin : gd initial $display("@G trunc def"); end endcase
  initial begin
    $display("@R %h %0d %h", W6'(P8), $bits(W6'(P8)), XB);
    case (6'h30) W6'(P8): $display("@P proc hit"); default: $display("@P proc def"); endcase
  end
endmodule
